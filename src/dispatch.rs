use crate::errors::new_error;
use crate::msg::EmptyResponse;
use crate::msg::ResourceId;
use crate::util::wrap_op;
use crate::util::serialize_response;
use crate::util::serialize_and_wrap;
use deno::PinnedBuf;
use deno::CoreOp;
use deno::Op;
use futures::sink::Sink;
use futures::sync::oneshot;
use futures::sync::mpsc::channel;
use futures::sync::mpsc::Sender;
use futures::sync::mpsc::Receiver;
use futures::stream::Stream;
use futures::stream::StreamFuture;
use futures::future::Future;
use futures::future::Shared;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::sync::RwLock;
use std::sync::Mutex;
use std::sync::Arc;

lazy_static! {
    static ref NEXT_DISPATCHER_ID: AtomicU32 = AtomicU32::new(0);
    static ref DISPATCHER_MAP: RwLock<HashMap<u32, Arc<Box<Dispatcher>>>> = RwLock::new(HashMap::new());
    static ref NEXT_STANDARD_DISPATCHER_ID: AtomicU32 = AtomicU32::new(0);
    static ref STANDARD_DISPATCHER_MAP: RwLock<HashMap<u32, Arc<StandardDispatcher>>> = RwLock::new(HashMap::new());
}

// TODO(afinch7) maybe move this to another package/crate
pub trait Dispatcher: Send + Sync {
    fn dispatch(&self, data: &[u8], zero_copy: Option<PinnedBuf>) -> CoreOp;
}

pub fn insert_dispatcher(dispatcher: Arc<Box<Dispatcher>>) -> ResourceId {
    let rid = NEXT_DISPATCHER_ID.fetch_add(1, Ordering::SeqCst);
    let mut lock = DISPATCHER_MAP.write().unwrap();
    lock.insert(rid, dispatcher);
    rid
}

pub fn get_dispatcher(dispatcher_rid: ResourceId) -> Arc<Box<Dispatcher>> {
    let lock = DISPATCHER_MAP.read().unwrap();
    let dispatcher_ref = lock.get(&dispatcher_rid).unwrap();
    dispatcher_ref.clone()
}

type StandardDispatchReq = (u32, Vec<u8>, Option<Vec<u8>>);
type StandardDispatchReqReceiver = Shared<StreamFuture<Receiver<StandardDispatchReq>>>;

struct StandardDispatcher {
    pub next_cmd_id: AtomicU32,
    pub res_senders: Arc<RwLock<HashMap<u32, oneshot::Sender<CoreOp>>>>,
    pub req_receiver: Arc<Mutex<StandardDispatchReqReceiver>>,
    pub req_sender: Sender<StandardDispatchReq>,
}

impl StandardDispatcher {
    pub fn new() -> Self {
        let (req_sender, req_reciever) = channel::<StandardDispatchReq>(1024);
        Self {
            next_cmd_id: AtomicU32::new(0),
            res_senders: Arc::new(RwLock::new(HashMap::new())),
            req_receiver: Arc::new(Mutex::new(req_reciever.into_future().shared())),
            req_sender:req_sender,
        }
    }
}

impl Dispatcher for StandardDispatcher {
    fn dispatch(&self, data: &[u8], zero_copy: Option<PinnedBuf>) -> CoreOp {
        let cmd_id = self.next_cmd_id.fetch_add(1, Ordering::SeqCst);
        let (res_sender, res_reciever) = oneshot::channel::<CoreOp>();
        let mut lock = self.res_senders.write().unwrap();
        lock.insert(cmd_id, res_sender);
        let sender_clone = self.req_sender.clone();
        sender_clone.send((cmd_id, data.to_vec(), zero_copy.map(|v| v.to_vec())));
        res_reciever.wait().unwrap()
    }
}

impl Dispatcher for Arc<StandardDispatcher> {
    fn dispatch(&self, data: &[u8], zero_copy: Option<PinnedBuf>) -> CoreOp {
        self.as_ref().dispatch(data, zero_copy)
    }
}

#[derive(Serialize)]
struct NewStandardDispatcherResponse {
    pub std_dispatcher_rid: u32,
    pub dispatcher_rid: u32,
}

pub fn op_new_standard_dispatcher(
    data: &[u8],
    zero_copy: Option<PinnedBuf>,
) -> CoreOp {
    wrap_op(|_data, _zero_copy| {
        let std_rid = NEXT_STANDARD_DISPATCHER_ID.fetch_add(1, Ordering::SeqCst);
        let dispatcher = Arc::new(StandardDispatcher::new());
        let mut lock = STANDARD_DISPATCHER_MAP.write().unwrap();
        lock.insert(std_rid, dispatcher.clone());
        let rid = insert_dispatcher(Arc::new(Box::new(dispatcher) as Box<Dispatcher>));
        
        serialize_and_wrap(NewStandardDispatcherResponse {
            std_dispatcher_rid: std_rid,
            dispatcher_rid: rid,
        })
    }, data, zero_copy)
}

lazy_static! {
    static ref NEXT_RES_SENDER_ID: AtomicU32 = AtomicU32::new(0);
    static ref RES_SENDER_MAP: RwLock<HashMap<u32, Arc<Box<Dispatcher>>>> = RwLock::new(HashMap::new());
}

#[derive(Deserialize)]
struct StandardDispatcherWaitForDispatchOptions {
    pub rid: u32,
    pub op_id: u32,
}

#[derive(Serialize)]
struct StandardDispatcherWaitForDispatchResponse {
    pub cmd_id: u32,
    pub data: Vec<u8>,
    pub zero_copy: Option<Vec<u8>>,
}

pub fn op_standard_dispatcher_wait_for_dispatch(
    data: &[u8],
    zero_copy: Option<PinnedBuf>,
) -> CoreOp {
    wrap_op(|data, _zero_copy| {
        let data_str = std::str::from_utf8(&data[..]).unwrap();
        let options: StandardDispatcherWaitForDispatchOptions = serde_json::from_str(data_str).unwrap();
        let lock = STANDARD_DISPATCHER_MAP.read().unwrap();
        let dispatcher = lock.get(&options.rid).unwrap();
        let recv_lock = dispatcher.req_receiver.lock().unwrap().clone();
        let op = Box::new(recv_lock
            .map_err(|err| new_error(&format!("{:#?}", err)))
            .and_then(|maybe_req| {
                match &maybe_req.0 {
                    // TODO(afinch7) the clones here are going to slow things down a lot.
                    Some(req) => Ok(serialize_response(StandardDispatcherWaitForDispatchResponse {
                        cmd_id: req.0,
                        data: req.1.clone(),
                        zero_copy: req.2.clone(),
                    })),
                    None => panic!("Recv stream ended!"),
                }
            })
        );
        Ok(Op::Async(op))
    }, data, zero_copy)
}

#[derive(Deserialize)]
struct StandardDispatcherRespondOptions {
    pub rid: u32,
    pub cmd_id: u32,
}

pub fn op_standard_dispatcher_respond(
    data: &[u8],
    zero_copy: Option<PinnedBuf>,
) -> CoreOp {
    wrap_op(|data, zero_copy| {
        let data_str = std::str::from_utf8(&data[..]).unwrap();
        let options: StandardDispatcherRespondOptions = serde_json::from_str(data_str).unwrap();
        let lock = STANDARD_DISPATCHER_MAP.read().unwrap();
        let dispatcher = lock.get(&options.rid).unwrap();
        let mut senders_lock = dispatcher.res_senders.write().unwrap();
        let sender = senders_lock.remove(&options.cmd_id).unwrap();
        match zero_copy {
            Some(buf) => {
                assert!(sender.send(Op::Sync(buf[..].into())).is_ok());
                serialize_and_wrap(EmptyResponse)
            },
            None => {
                panic!("Promise returns not implemented yet!");
                // TODO(afinch7) implement promise returns.
            }
        }
    }, data, zero_copy)
}