use crate::errors::DIDResult;
use crate::msg::EmptyResponse;
use crate::msg::ResourceId;
use crate::util::DIDOp;
use crate::util::wrap_op;
use crate::util::serialize_response;
use crate::util::serialize_sync_result;
use deno::PinnedBuf;
use deno::CoreOp;
use deno::Op;
use deno::Buf;
use futures::channel::oneshot;
use futures::future::FutureExt;
use futures::task::AtomicWaker;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::future::Future;
use std::task::Context;
use std::task::Poll;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::sync::RwLock;
use std::sync::Mutex;
use std::sync::Arc;
use std::pin::Pin;

lazy_static! {
    static ref NEXT_DISPATCHER_ID: AtomicU32 = AtomicU32::new(1);
    static ref DISPATCHER_MAP: RwLock<HashMap<u32, Arc<Box<dyn Dispatcher>>>> = RwLock::new(HashMap::new());
    static ref NEXT_STANDARD_DISPATCHER_ID: AtomicU32 = AtomicU32::new(1);
    static ref STANDARD_DISPATCHER_MAP: RwLock<HashMap<u32, Arc<StandardDispatcher>>> = RwLock::new(HashMap::new());
}

// TODO(afinch7) maybe move this to another package/crate
pub trait Dispatcher: Send + Sync {
    fn dispatch(&self, data: &[u8], zero_copy: Option<PinnedBuf>) -> CoreOp;
}

pub fn insert_dispatcher(dispatcher: Arc<Box<dyn Dispatcher>>) -> ResourceId {
    let rid = NEXT_DISPATCHER_ID.fetch_add(1, Ordering::SeqCst);
    let mut lock = DISPATCHER_MAP.write().unwrap();
    lock.insert(rid, dispatcher);
    rid
}

pub fn get_dispatcher(dispatcher_rid: ResourceId) -> Arc<Box<dyn Dispatcher>> {
    let lock = DISPATCHER_MAP.read().unwrap();
    let dispatcher_ref = lock.get(&dispatcher_rid).unwrap();
    dispatcher_ref.clone()
}

type StandardDispatchReq = (u32, Vec<u8>, Option<Vec<u8>>);
type StandardDispatchReqQueue= VecDeque<StandardDispatchReq>;

struct StandardDispatcher {
    pub next_cmd_id: AtomicU32,
    pub res_senders: Arc<RwLock<HashMap<u32, oneshot::Sender<CoreOp>>>>,
    pub req_queue: Arc<Mutex<StandardDispatchReqQueue>>,
    pub waker: AtomicWaker,
}

impl StandardDispatcher {
    pub fn new() -> Self {
        Self {
            next_cmd_id: AtomicU32::new(0),
            res_senders: Arc::new(RwLock::new(HashMap::new())),
            req_queue: Arc::new(Mutex::new(VecDeque::new())),
            waker: AtomicWaker::new(),
        }
    }
}

impl Dispatcher for StandardDispatcher {
    fn dispatch(&self, data: &[u8], zero_copy: Option<PinnedBuf>) -> CoreOp {
        let cmd_id = self.next_cmd_id.fetch_add(1, Ordering::SeqCst);
        let (res_sender, res_reciever) = oneshot::channel::<CoreOp>();
        {
            let mut lock = self.res_senders.write().unwrap();
            lock.insert(cmd_id, res_sender);
            let mut queue = self.req_queue.lock().unwrap();
            queue.push_back((cmd_id, data.to_vec(), zero_copy.map(|v| v.to_vec())));
        }
        self.waker.wake();
        futures::executor::block_on(res_reciever).unwrap()
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
        let rid = insert_dispatcher(Arc::new(Box::new(dispatcher) as Box<dyn Dispatcher>));
        
        serialize_sync_result(NewStandardDispatcherResponse {
            std_dispatcher_rid: std_rid,
            dispatcher_rid: rid,
        })
    }, data, zero_copy)
}

#[derive(Deserialize)]
struct StandardDispatcherWaitForDispatchOptions {
    pub rid: u32,
}

#[derive(Serialize)]
struct StandardDispatcherWaitForDispatchResponse {
    pub cmd_id: u32,
    // TODO(afinch7) encode these outside of json.
    // Currently we encode data and zero_copy into json.
    // This will be very slow, but it works for testing.
    pub data: Vec<u8>,
    pub zero_copy: Option<Vec<u8>>,
}

struct RecvWorker {
    pub rid: u32,
}

impl Future for RecvWorker {
    type Output = DIDResult<Buf>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let lock = STANDARD_DISPATCHER_MAP.read().unwrap();
        let dispatcher = lock.get(&self.rid).unwrap();
        dispatcher.waker.register(cx.waker());
        let mut queue = dispatcher.req_queue.lock().unwrap();
        let result = match queue.pop_front() {
            Some(req) => Poll::Ready(Ok(serialize_response(StandardDispatcherWaitForDispatchResponse {
                cmd_id: req.0,
                data: req.1,
                zero_copy: req.2,
            }))),
            None => Poll::Pending,
        };
        result
    }
}

pub fn op_standard_dispatcher_wait_for_dispatch(
    data: &[u8],
    zero_copy: Option<PinnedBuf>,
) -> CoreOp {
    wrap_op(|data, _zero_copy| {
        let data_str = std::str::from_utf8(&data[..]).unwrap();
        let options: StandardDispatcherWaitForDispatchOptions = serde_json::from_str(data_str).unwrap();
        
        let op = RecvWorker {
            rid: options.rid,
        };

        Ok(DIDOp::Async(op.boxed()))
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
                serialize_sync_result(EmptyResponse)
            },
            None => {
                panic!("Promise returns not implemented yet!");
                // TODO(afinch7) implement promise returns.
            }
        }
    }, data, zero_copy)
}