use crate::errors::new_error;
use crate::errors::DIDResult;
use crate::msg::EmptyResponse;
use crate::msg::ResourceId;
use crate::msg::ResourceIdResponse;
use crate::util::wrap_op;
use crate::util::serialize_sync_result;
use crate::util::serialize_response;
use crate::util::DIDOp;
use deno::CoreOp;
use deno::Buf;
use deno::ErrBox;
use deno::ModuleSpecifier;
use deno::SourceCodeInfoFuture;
use deno::SourceCodeInfo;
use deno::PinnedBuf;
use deno::Loader;
use deno::Modules;
use futures::future::FutureExt;
use futures::channel::oneshot;
use futures::task::AtomicWaker;
use serde::Serialize;
use serde::Deserialize;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::future::Future;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::sync::RwLock;
use std::sync::Mutex;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;
use std::pin::Pin;

lazy_static! {
    static ref NEXT_LOADER_ID: AtomicU32 = AtomicU32::new(1);
    static ref LOADER_MAP: RwLock<HashMap<u32, Arc<Box<dyn Loader>>>> = RwLock::new(HashMap::new());
    static ref NEXT_STD_LOADER_ID: AtomicU32 = AtomicU32::new(1);
    static ref STD_LOADER_MAP: RwLock<HashMap<u32, Arc<StdLoader>>> = RwLock::new(HashMap::new());
    static ref NEXT_MODULE_STORE_ID: AtomicU32 = AtomicU32::new(1);
    static ref MODULE_STORE_MAP: RwLock<HashMap<u32, Arc<Mutex<Modules>>>> = RwLock::new(HashMap::new());
}

struct LoaderWrapper {
    pub inner: Arc<Box<dyn Loader>>,
}

impl Loader for LoaderWrapper {
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        is_root: bool,
    ) -> Result<ModuleSpecifier, ErrBox> {
        self.inner.as_ref().resolve(specifier, referrer, is_root)
    }

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
    ) -> Pin<Box<SourceCodeInfoFuture>> {
        self.inner.as_ref().load(module_specifier)
    }
}

pub fn get_module_store(module_store_rid: ResourceId) -> Arc<Mutex<Modules>> {
    let lock = MODULE_STORE_MAP.read().unwrap();
    let module_store_ref = lock.get(&module_store_rid).unwrap();
    Arc::clone(module_store_ref)
}

pub fn insert_loader(loader: Arc<Box<dyn Loader>>) -> ResourceId {
    let rid = NEXT_LOADER_ID.fetch_add(1, Ordering::SeqCst);
    let mut lock = LOADER_MAP.write().unwrap();
    lock.insert(rid, loader);
    rid
}

pub fn get_loader(loader_rid: ResourceId) -> impl Loader {
    let lock = LOADER_MAP.read().unwrap();
    let loader_ref = lock.get(&loader_rid).unwrap();
    LoaderWrapper {
        inner: Arc::clone(loader_ref),
    }
}

pub fn op_new_module_store(
    data: &[u8],
    zero_copy: Option<PinnedBuf>,
) -> CoreOp {
    wrap_op(|_data, _zero_copy| {
        let module_store = Arc::new(Mutex::new(Modules::new()));
        let rid = NEXT_MODULE_STORE_ID.fetch_add(1, Ordering::SeqCst);
        let mut lock = MODULE_STORE_MAP.write().unwrap();
        lock.insert(rid, module_store);
        serialize_sync_result(ResourceIdResponse {
            rid,
        })
    }, data, zero_copy)
}

type StdLoaderResolveReq = (u32, String, String, bool);
type StdLoaderResolveReqQueue = VecDeque<StdLoaderResolveReq>;
type StdLoaderResolveRes = Result<ModuleSpecifier, ErrBox>;

type StdLoaderLoadReq = (u32, String);
type StdLoaderLoadReqQueue = VecDeque<StdLoaderLoadReq>;
type StdLoaderLoadRes = Result<SourceCodeInfo, ErrBox>;

// TODO(afinch7) maybe break this into two structs Resolver + Loader
pub struct StdLoader {
    pub next_resolve_id: AtomicU32,
    pub resolve_res_senders: Arc<RwLock<HashMap<u32, oneshot::Sender<StdLoaderResolveRes>>>>,
    pub resolve_req_queue: Arc<Mutex<StdLoaderResolveReqQueue>>,
    pub resolve_waker: AtomicWaker,
    pub next_load_id: AtomicU32,
    pub load_res_senders: Arc<RwLock<HashMap<u32, oneshot::Sender<StdLoaderLoadRes>>>>,
    pub load_req_queue: Arc<Mutex<StdLoaderLoadReqQueue>>,
    pub load_waker: AtomicWaker,
}

impl StdLoader {
    pub fn new() -> Self {
        Self {
            next_resolve_id: AtomicU32::new(0),
            resolve_res_senders: Arc::new(RwLock::new(HashMap::new())),
            resolve_req_queue: Arc::new(Mutex::new(VecDeque::new())),
            resolve_waker: AtomicWaker::new(),
            next_load_id: AtomicU32::new(0),
            load_res_senders: Arc::new(RwLock::new(HashMap::new())),
            load_req_queue: Arc::new(Mutex::new(VecDeque::new())),
            load_waker: AtomicWaker::new(),
        }
    }
}

impl Loader for StdLoader {
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        is_root: bool,
    ) -> Result<ModuleSpecifier, ErrBox> {
        let cmd_id = self.next_resolve_id.fetch_add(1, Ordering::SeqCst);
        let (res_sender, res_reciever) = oneshot::channel::<StdLoaderResolveRes>();
        {
            let mut lock = self.resolve_res_senders.write().unwrap();
            lock.insert(cmd_id, res_sender);
            let mut queue = self.resolve_req_queue.lock().unwrap();
            queue.push_back((cmd_id, specifier.to_string(), referrer.to_string(), is_root));
        }
        self.resolve_waker.wake();
        futures::executor::block_on(res_reciever).unwrap()
    }

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
    ) -> Pin<Box<SourceCodeInfoFuture>> {
        let cmd_id = self.next_load_id.fetch_add(1, Ordering::SeqCst);
        let (res_sender, res_reciever) = oneshot::channel::<StdLoaderLoadRes>();
        {
            let mut lock = self.load_res_senders.write().unwrap();
            lock.insert(cmd_id, res_sender);
            let mut queue = self.load_req_queue.lock().unwrap();
            queue.push_back((cmd_id, module_specifier.as_url().to_string()));
        }
        self.load_waker.wake();
        res_reciever.map(|r| r.unwrap()).boxed()
    }
}

struct StdLoaderArcWrapper {
    pub inner: Arc<StdLoader>,
}

impl Loader for StdLoaderArcWrapper {
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        is_root: bool,
    ) -> Result<ModuleSpecifier, ErrBox> {
        self.inner.as_ref().resolve(specifier, referrer, is_root)
    }

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
    ) -> Pin<Box<SourceCodeInfoFuture>> {
        self.inner.as_ref().load(module_specifier)
    }
}

#[derive(Serialize)]
struct NewStdDispatcherResponse {
    pub std_loader_rid: u32,
    pub loader_rid: u32,
}

pub fn op_new_std_loader(
    data: &[u8],
    zero_copy: Option<PinnedBuf>,
) -> CoreOp {
    wrap_op(|_data, _zero_copy| {
        let std_rid = NEXT_STD_LOADER_ID.fetch_add(1, Ordering::SeqCst);
        let loader = Arc::new(StdLoader::new());
        let mut lock = STD_LOADER_MAP.write().unwrap();
        lock.insert(std_rid, Arc::clone(&loader));
        let rid = insert_loader(Arc::new(Box::new(StdLoaderArcWrapper {
            inner: loader,
        }) as Box<dyn Loader>));

        serialize_sync_result(NewStdDispatcherResponse {
            std_loader_rid: std_rid,
            loader_rid: rid,
        })
    }, data, zero_copy)
}


#[derive(Deserialize)]
struct StdLoaderAwaitResolveOptions {
    pub rid: u32,
}

#[derive(Serialize)]
struct StdLoaderAwaitResolveResponse {
    pub cmd_id: u32,
    pub specifier: String,
    pub referrer: String,
    pub is_root: bool,
}

struct ResolveWorker {
    pub rid: u32,
}

impl Future for ResolveWorker {
    type Output = DIDResult<Buf>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let lock = STD_LOADER_MAP.read().unwrap();
        let loader = lock.get(&self.rid).unwrap();
        loader.resolve_waker.register(cx.waker());
        let mut queue = loader.resolve_req_queue.lock().unwrap();
        let result = match queue.pop_front() {
            Some(req) => Poll::Ready(Ok(serialize_response(StdLoaderAwaitResolveResponse {
                cmd_id: req.0,
                specifier: req.1,
                referrer: req.2,
                is_root: req.3,
            }))),
            None => Poll::Pending,
        };
        result
    }
}

pub fn op_std_loader_await_resolve(
    data: &[u8],
    zero_copy: Option<PinnedBuf>,
) -> CoreOp {
    wrap_op(|data, _zero_copy| {
        let data_str = std::str::from_utf8(&data[..]).unwrap();
        let options: StdLoaderAwaitResolveOptions = serde_json::from_str(data_str).unwrap();

        let op = ResolveWorker {
            rid: options.rid,
        };

        Ok(DIDOp::Async(op.boxed()))
    }, data, zero_copy)
}

#[derive(Deserialize)]
struct StdLoaderRespondResolveOptions {
    pub rid: u32,
    pub cmd_id: u32,
    pub module_specifier: String,
}

pub fn op_std_loader_respond_resolve(
    data: &[u8],
    zero_copy: Option<PinnedBuf>,
) -> CoreOp {
    wrap_op(|data, _zero_copy| {
        let data_str = std::str::from_utf8(&data[..]).unwrap();
        let options: StdLoaderRespondResolveOptions = serde_json::from_str(data_str).unwrap();
        let lock = STD_LOADER_MAP.read().unwrap();
        let loader = lock.get(&options.rid).unwrap();
        let mut senders_lock = loader.resolve_res_senders.write().unwrap();
        let sender = senders_lock.remove(&options.cmd_id).unwrap();
        let result = ModuleSpecifier::resolve_url(&options.module_specifier)
            .map_err(ErrBox::from);
        let js_result = match &result {
            Ok(_) => serialize_sync_result(EmptyResponse),
            Err(err) => Err(new_error(&format!("{:#?}", err))),
        };
        sender.send(result).unwrap();
        js_result
    }, data, zero_copy)
}


#[derive(Deserialize)]
struct StdLoaderAwaitLoadOptions {
    pub rid: u32,
}

#[derive(Serialize)]
struct StdLoaderAwaitLoadResponse {
    pub cmd_id: u32,
    pub module_specifier: String,
}

struct LoadWorker {
    pub rid: u32,
}

impl Future for LoadWorker {
    type Output = DIDResult<Buf>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let lock = STD_LOADER_MAP.read().unwrap();
        let loader = lock.get(&self.rid).unwrap();
        loader.load_waker.register(cx.waker());
        let mut queue = loader.load_req_queue.lock().unwrap();
        let result = match queue.pop_front() {
            Some(req) => Poll::Ready(Ok(serialize_response(StdLoaderAwaitLoadResponse {
                cmd_id: req.0,
                module_specifier: req.1,
            }))),
            None => Poll::Pending,
        };
        result
    }
}

pub fn op_std_loader_await_load(
    data: &[u8],
    zero_copy: Option<PinnedBuf>,
) -> CoreOp {
    wrap_op(|data, _zero_copy| {
        let data_str = std::str::from_utf8(&data[..]).unwrap();
        let options: StdLoaderAwaitResolveOptions = serde_json::from_str(data_str).unwrap();

        let op = LoadWorker {
            rid: options.rid,
        };

        Ok(DIDOp::Async(op.boxed()))
    }, data, zero_copy)
}

#[derive(Deserialize)]
struct StdLoaderRespondLoadOptions {
    pub rid: u32,
    pub cmd_id: u32,
    pub module_name: String,
    pub code: String,
}

pub fn op_std_loader_respond_load(
    data: &[u8],
    zero_copy: Option<PinnedBuf>,
) -> CoreOp {
    wrap_op(|data, _zero_copy| {
        let data_str = std::str::from_utf8(&data[..]).unwrap();
        let options: StdLoaderRespondLoadOptions = serde_json::from_str(data_str).unwrap();
        let lock = STD_LOADER_MAP.read().unwrap();
        let loader = lock.get(&options.rid).unwrap();
        let mut senders_lock = loader.load_res_senders.write().unwrap();
        let sender = senders_lock.remove(&options.cmd_id).unwrap();
        assert!(sender.send(Ok(SourceCodeInfo {
            module_name: options.module_name,
            code: options.code,
        })).is_ok());
        serialize_sync_result(EmptyResponse)
    }, data, zero_copy)
}