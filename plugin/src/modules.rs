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
    static ref NEXT_LOADER_ID: AtomicU32 = AtomicU32::new(1);
    static ref LOADER_MAP: RwLock<HashMap<u32, Arc<Box<dyn Loader>>>> = RwLock::new(HashMap::new());
    static ref NEXT_STD_LOADER_ID: AtomicU32 = AtomicU32::new(1);
    static ref STD_LOADER_MAP: RwLock<HashMap<u32, Arc<StdLoader>>> = RwLock::new(HashMap::new());
}

struct LoaderWrapper {
    pub inner: Arc<Box<dyn Loader>>,
}

impl Loader for LoaderWrapper {
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        is_main: bool,
        is_dyn_import: bool,
    ) -> Result<ModuleSpecifier, ErrBox> {
        self.inner
            .as_ref()
            .resolve(specifier, referrer, is_main, is_dyn_import)
    }

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        maybe_referrer: Option<ModuleSpecifier>,
    ) -> Pin<Box<SourceCodeInfoFuture>> {
        self.inner.as_ref().load(module_specifier, maybe_referrer)
    }
}

pub fn insert_loader(loader: Arc<Box<dyn Loader>>) -> ResourceId {
    let rid = NEXT_LOADER_ID.fetch_add(1, Ordering::SeqCst);
    let mut lock = LOADER_MAP.write().unwrap();
    lock.insert(rid, loader);
    rid
}

pub fn get_loader(loader_rid: ResourceId) -> Box<dyn Loader + Unpin> {
    let lock = LOADER_MAP.read().unwrap();
    let loader_ref = lock.get(&loader_rid).unwrap();
    Box::new(LoaderWrapper {
        inner: Arc::clone(loader_ref),
    })
}

type StdLoaderResolveReq = (u32, String, String, bool, bool);
type StdLoaderResolveReqQueue = VecDeque<StdLoaderResolveReq>;
type StdLoaderResolveRes = Result<ModuleSpecifier, ErrBox>;

type StdLoaderLoadReq = (u32, String, Option<String>);
type StdLoaderLoadReqQueue = VecDeque<StdLoaderLoadReq>;
type StdLoaderLoadRes = Result<SourceCodeInfo, ErrBox>;

// TODO(afinch7) maybe break this into two structs Resolver + Loader
pub struct StdLoader {
    pub next_resolve_id: AtomicU32,
    pub resolve_res_senders: Arc<RwLock<HashMap<u32, oneshot::Sender<StdLoaderResolveRes>>>>,
    pub resolve_req_queue: Arc<Mutex<StdLoaderResolveReqQueue>>,
    pub resolve_waker: AtomicWaker,
    pub next_load_id: AtomicU32,
    pub load_res_senders: Arc<RwLock<HashMap<u32, (String, oneshot::Sender<StdLoaderLoadRes>)>>>,
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
        is_main: bool,
        is_dyn_import: bool,
    ) -> Result<ModuleSpecifier, ErrBox> {
        let cmd_id = self.next_resolve_id.fetch_add(1, Ordering::SeqCst);
        let (res_sender, mut res_reciever) = oneshot::channel::<StdLoaderResolveRes>();
        {
            let mut lock = self.resolve_res_senders.write().unwrap();
            lock.insert(cmd_id, res_sender);
            let mut queue = self.resolve_req_queue.lock().unwrap();
            queue.push_back((
                cmd_id,
                specifier.to_string(),
                referrer.to_string(),
                is_main,
                is_dyn_import,
            ));
        }
        self.resolve_waker.wake();
        // TODO(afinch7) This is a realy ugly hack. Find a better solution here.
        let mut cx = futures::task::Context::from_waker(futures::task::noop_waker_ref());
        loop {
            match res_reciever.poll_unpin(&mut cx) {
                Poll::Ready(v) => return v.unwrap(),
                Poll::Pending => {}
            };
        }
    }

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        maybe_referrer: Option<ModuleSpecifier>,
    ) -> Pin<Box<SourceCodeInfoFuture>> {
        let cmd_id = self.next_load_id.fetch_add(1, Ordering::SeqCst);
        let (res_sender, res_reciever) = oneshot::channel::<StdLoaderLoadRes>();
        {
            let mut lock = self.load_res_senders.write().unwrap();
            let module_url_specified = module_specifier.as_url().to_string();
            lock.insert(cmd_id, (module_url_specified.clone(), res_sender));
            let mut queue = self.load_req_queue.lock().unwrap();
            queue.push_back((
                cmd_id,
                module_url_specified,
                maybe_referrer.map(|m| m.as_url().to_string()),
            ));
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
        is_main: bool,
        is_dyn_import: bool,
    ) -> Result<ModuleSpecifier, ErrBox> {
        self.inner
            .as_ref()
            .resolve(specifier, referrer, is_main, is_dyn_import)
    }

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        maybe_referrer: Option<ModuleSpecifier>,
    ) -> Pin<Box<SourceCodeInfoFuture>> {
        self.inner.as_ref().load(module_specifier, maybe_referrer)
    }
}

#[derive(Serialize)]
struct NewStdDispatcherResponse {
    pub std_loader_rid: u32,
    pub loader_rid: u32,
}

pub fn op_new_std_loader(_args: Value, _zero_copy: Option<ZeroCopyBuf>) -> Result<JsonOp, ErrBox> {
    let std_rid = NEXT_STD_LOADER_ID.fetch_add(1, Ordering::SeqCst);
    let loader = Arc::new(StdLoader::new());
    let mut lock = STD_LOADER_MAP.write().unwrap();
    lock.insert(std_rid, Arc::clone(&loader));
    let rid = insert_loader(Arc::new(
        Box::new(StdLoaderArcWrapper { inner: loader }) as Box<dyn Loader>
    ));

    Ok(JsonOp::Sync(json!(NewStdDispatcherResponse {
        std_loader_rid: std_rid,
        loader_rid: rid,
    })))
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
    type Output = Result<Value, ErrBox>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let lock = STD_LOADER_MAP.read().unwrap();
        let loader = lock.get(&self.rid).unwrap();
        loader.resolve_waker.register(cx.waker());
        let mut queue = loader.resolve_req_queue.lock().unwrap();
        let result = match queue.pop_front() {
            Some(req) => Poll::Ready(Ok(json!(StdLoaderAwaitResolveResponse {
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
    args: Value,
    _zero_copy: Option<ZeroCopyBuf>,
) -> Result<JsonOp, ErrBox> {
    let args: StdLoaderAwaitResolveOptions = serde_json::from_value(args)?;

    let op = ResolveWorker { rid: args.rid };

    Ok(JsonOp::Async(op.boxed()))
}

#[derive(Deserialize)]
struct StdLoaderRespondResolveOptions {
    pub rid: u32,
    pub cmd_id: u32,
    pub module_specifier: String,
}

pub fn op_std_loader_respond_resolve(
    args: Value,
    _zero_copy: Option<ZeroCopyBuf>,
) -> Result<JsonOp, ErrBox> {
    let args: StdLoaderRespondResolveOptions = serde_json::from_value(args)?;

    let lock = STD_LOADER_MAP.read().unwrap();
    let loader = lock.get(&args.rid).unwrap();
    let mut senders_lock = loader.resolve_res_senders.write().unwrap();
    let sender = senders_lock.remove(&args.cmd_id).unwrap();
    let result = ModuleSpecifier::resolve_url(&args.module_specifier);
    let js_result = match &result {
        Ok(_) => Ok(JsonOp::Sync(json!({}))),
        Err(err) => Err(ErrBox::from(err.clone())),
    };
    sender.send(result.map_err(ErrBox::from)).unwrap();
    js_result
}

#[derive(Deserialize)]
struct StdLoaderAwaitLoadOptions {
    pub rid: u32,
}

#[derive(Serialize)]
struct StdLoaderAwaitLoadResponse {
    pub cmd_id: u32,
    pub module_specifier: String,
    pub maybe_referrer: Option<String>,
}

struct LoadWorker {
    pub rid: u32,
}

impl Future for LoadWorker {
    type Output = Result<Value, ErrBox>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let lock = STD_LOADER_MAP.read().unwrap();
        let loader = lock.get(&self.rid).unwrap();
        loader.load_waker.register(cx.waker());
        let mut queue = loader.load_req_queue.lock().unwrap();
        let result = match queue.pop_front() {
            Some(req) => Poll::Ready(Ok(json!(StdLoaderAwaitLoadResponse {
                cmd_id: req.0,
                module_specifier: req.1,
                maybe_referrer: req.2
            }))),
            None => Poll::Pending,
        };
        result
    }
}

pub fn op_std_loader_await_load(
    args: Value,
    _zero_copy: Option<ZeroCopyBuf>,
) -> Result<JsonOp, ErrBox> {
    let args: StdLoaderAwaitResolveOptions = serde_json::from_value(args)?;

    let op = LoadWorker { rid: args.rid };

    Ok(JsonOp::Async(op.boxed()))
}

#[derive(Deserialize)]
struct StdLoaderRespondLoadOptions {
    pub rid: u32,
    pub cmd_id: u32,
    pub module_name: String,
    pub code: String,
}

pub fn op_std_loader_respond_load(
    args: Value,
    _zero_copy: Option<ZeroCopyBuf>,
) -> Result<JsonOp, ErrBox> {
    let args: StdLoaderRespondLoadOptions = serde_json::from_value(args)?;

    let lock = STD_LOADER_MAP.read().unwrap();
    let loader = lock.get(&args.rid).unwrap();
    let mut senders_lock = loader.load_res_senders.write().unwrap();
    let (module_url_specified, sender) = senders_lock.remove(&args.cmd_id).unwrap();
    assert!(sender
        .send(Ok(SourceCodeInfo {
            module_url_specified,
            module_url_found: args.module_name,
            code: args.code,
        }))
        .is_ok());
    Ok(JsonOp::Sync(json!({})))
}
