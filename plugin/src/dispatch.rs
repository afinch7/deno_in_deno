use crate::msg::ResourceId;
use deno_core::*;
use deno_dispatch_json::JsonOp;
use futures::channel::oneshot;
use futures::future::FutureExt;
use futures::task::AtomicWaker;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use std::task::Context;
use std::task::Poll;

lazy_static! {
    static ref NEXT_DISPATCHER_ID: AtomicU32 = AtomicU32::new(1);
    static ref DISPATCHER_MAP: RwLock<HashMap<u32, Arc<Box<dyn Dispatcher>>>> =
        RwLock::new(HashMap::new());
    static ref NEXT_STD_DISPATCHER_ID: AtomicU32 = AtomicU32::new(1);
    static ref STD_DISPATCHER_MAP: RwLock<HashMap<u32, Arc<StdDispatcher>>> =
        RwLock::new(HashMap::new());
}

// TODO(afinch7) maybe move this to another package/crate
pub trait Dispatcher: Send + Sync {
    fn dispatch(&self, data: &[u8], zero_copy: Option<ZeroCopyBuf>) -> CoreOp;
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

pub type InsertDispatcherAccessor = fn(Arc<Box<dyn Dispatcher>>) -> ResourceId;
pub type GetDispatcherAccessor = fn(ResourceId) -> Arc<Box<dyn Dispatcher>>;

#[derive(Serialize)]
struct GetDispatcherAccessorPtrResponse {
    pub get_dispatcher_ptr: usize,
    pub insert_dispatcher_ptr: usize,
}

pub fn op_get_dispatcher_accessor_ptrs(
    _args: Value,
    _zero_copy: Option<ZeroCopyBuf>,
) -> Result<JsonOp, ErrBox> {
    let get_dispatcher_ptr: usize =
        &(get_dispatcher as GetDispatcherAccessor) as *const GetDispatcherAccessor as usize;
    let insert_dispatcher_ptr: usize = &(insert_dispatcher as InsertDispatcherAccessor)
        as *const InsertDispatcherAccessor as usize;
    Ok(JsonOp::Sync(json!(GetDispatcherAccessorPtrResponse {
        get_dispatcher_ptr,
        insert_dispatcher_ptr,
    })))
}

type StdDispatchReq = (u32, Vec<u8>, Option<Vec<u8>>);
type StdDispatchReqQueue = VecDeque<StdDispatchReq>;

struct StdDispatcher {
    pub next_cmd_id: AtomicU32,
    pub res_senders: Arc<RwLock<HashMap<u32, oneshot::Sender<CoreOp>>>>,
    pub req_queue: Arc<Mutex<StdDispatchReqQueue>>,
    pub waker: AtomicWaker,
}

impl StdDispatcher {
    pub fn new() -> Self {
        Self {
            next_cmd_id: AtomicU32::new(0),
            res_senders: Arc::new(RwLock::new(HashMap::new())),
            req_queue: Arc::new(Mutex::new(VecDeque::new())),
            waker: AtomicWaker::new(),
        }
    }
}

impl Dispatcher for StdDispatcher {
    fn dispatch(&self, data: &[u8], zero_copy: Option<ZeroCopyBuf>) -> CoreOp {
        let cmd_id = self.next_cmd_id.fetch_add(1, Ordering::SeqCst);
        let (res_sender, mut res_reciever) = oneshot::channel::<CoreOp>();
        {
            let mut lock = self.res_senders.write().unwrap();
            lock.insert(cmd_id, res_sender);
            let mut queue = self.req_queue.lock().unwrap();
            queue.push_back((cmd_id, data.to_vec(), zero_copy.map(|v| v.to_vec())));
        }
        self.waker.wake();
        // TODO(afinch7) This is a realy ugly hack. Find a better solution here.
        let mut cx = futures::task::Context::from_waker(futures::task::noop_waker_ref());
        loop {
            match res_reciever.poll_unpin(&mut cx) {
                Poll::Ready(v) => return v.unwrap(),
                Poll::Pending => {}
            };
        }
    }
}

impl Dispatcher for Arc<StdDispatcher> {
    fn dispatch(&self, data: &[u8], zero_copy: Option<ZeroCopyBuf>) -> CoreOp {
        self.as_ref().dispatch(data, zero_copy)
    }
}

#[derive(Serialize)]
struct NewStdDispatcherResponse {
    pub std_dispatcher_rid: u32,
    pub dispatcher_rid: u32,
}

pub fn op_new_std_dispatcher(
    _args: Value,
    _zero_copy: Option<ZeroCopyBuf>,
) -> Result<JsonOp, ErrBox> {
    let std_rid = NEXT_STD_DISPATCHER_ID.fetch_add(1, Ordering::SeqCst);
    let dispatcher = Arc::new(StdDispatcher::new());
    let mut lock = STD_DISPATCHER_MAP.write().unwrap();
    lock.insert(std_rid, dispatcher.clone());
    let rid = insert_dispatcher(Arc::new(Box::new(dispatcher) as Box<dyn Dispatcher>));

    Ok(JsonOp::Sync(json!(NewStdDispatcherResponse {
        std_dispatcher_rid: std_rid,
        dispatcher_rid: rid,
    })))
}

#[derive(Deserialize)]
struct StdDispatcherWaitForDispatchOptions {
    pub rid: u32,
}

#[derive(Serialize)]
struct StdDispatcherWaitForDispatchResponse {
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
    type Output = Result<Value, ErrBox>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let lock = STD_DISPATCHER_MAP.read().unwrap();
        let dispatcher = lock.get(&self.rid).unwrap();
        dispatcher.waker.register(cx.waker());
        let mut queue = dispatcher.req_queue.lock().unwrap();
        let result = match queue.pop_front() {
            Some(req) => Poll::Ready(Ok(json!(StdDispatcherWaitForDispatchResponse {
                cmd_id: req.0,
                data: req.1,
                zero_copy: req.2,
            }))),
            None => Poll::Pending,
        };
        result
    }
}

pub fn op_std_dispatcher_wait_for_dispatch(
    args: Value,
    _zero_copy: Option<ZeroCopyBuf>,
) -> Result<JsonOp, ErrBox> {
    let args: StdDispatcherWaitForDispatchOptions = serde_json::from_value(args)?;

    let op = RecvWorker { rid: args.rid };

    Ok(JsonOp::Async(op.boxed()))
}

#[derive(Deserialize)]
struct StdDispatcherRespondOptions {
    pub rid: u32,
    pub cmd_id: u32,
}

pub fn op_std_dispatcher_respond(
    args: Value,
    zero_copy: Option<ZeroCopyBuf>,
) -> Result<JsonOp, ErrBox> {
    let args: StdDispatcherRespondOptions = serde_json::from_value(args)?;
    let lock = STD_DISPATCHER_MAP.read().unwrap();
    let dispatcher = lock.get(&args.rid).unwrap();
    let mut senders_lock = dispatcher.res_senders.write().unwrap();
    let sender = senders_lock.remove(&args.cmd_id).unwrap();
    match zero_copy {
        Some(buf) => {
            assert!(sender.send(Op::Sync(buf[..].into())).is_ok());
            Ok(JsonOp::Sync(json!({})))
        }
        None => {
            panic!("Promise returns not implemented yet!");
            // TODO(afinch7) implement promise returns.
        }
    }
}
