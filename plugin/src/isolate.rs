use crate::dispatch::get_dispatcher;
use crate::modules::get_loader;
use crate::msg::ResourceId;
use crate::msg::ResourceIdResponse;
use deno_core::*;
use deno_dispatch_json::JsonOp;
use futures::future::FutureExt;
use futures::future::TryFutureExt;
use futures::task::AtomicWaker;
use futures::task::Context;
use futures::task::Poll;
use futures::task::SpawnExt;
use serde::Deserialize;
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::RwLock;
use tokio::sync::Mutex;

lazy_static! {
    static ref NEXT_ISOLATE_ID: AtomicU32 = AtomicU32::new(1);
    static ref ISOLATE_MAP: RwLock<HashMap<u32, Arc<Mutex<Box<EsIsolate>>>>> =
        RwLock::new(HashMap::new());
}

#[derive(Deserialize)]
struct NewIsolateOptions {
    pub will_snapshot: bool,
    pub snapshot_rid: Option<u32>,
    pub loader_rid: u32,
}

pub fn op_new_isolate(args: Value, _zero_copy: Option<PinnedBuf>) -> Result<JsonOp, ErrBox> {
    let args: NewIsolateOptions = serde_json::from_value(args)?;

    let rid = match args.snapshot_rid {
        Some(rid) => {
            let startup_data = crate::snapshots::snapshot_as_startup_data(rid);
            let isolate_rid =
                op_new_isolate_inner(args.loader_rid, startup_data, args.will_snapshot);
            isolate_rid
        }
        None => {
            let isolate_rid =
                op_new_isolate_inner(args.loader_rid, StartupData::None, args.will_snapshot);
            isolate_rid
        }
    };
    Ok(JsonOp::Sync(json!(ResourceIdResponse { rid })))
    // TODO(afinch7) figure out some way to handle startup data.
}

fn op_new_isolate_inner(
    loader_rid: u32,
    startup_data: StartupData,
    will_snapshot: bool,
) -> ResourceId {
    let isolate_rid = NEXT_ISOLATE_ID.fetch_add(1, Ordering::SeqCst);
    let loader = get_loader(loader_rid);
    let isolate = EsIsolate::new(loader, startup_data, will_snapshot);
    let mut lock = ISOLATE_MAP.write().unwrap();
    lock.insert(isolate_rid, Arc::new(Mutex::new(isolate)));
    isolate_rid
}

#[derive(Deserialize)]
struct IsolateIsCompleteOptions {
    pub rid: u32,
}

struct IsolateWorker {
    pub isolate: Arc<Mutex<Box<EsIsolate>>>,
}

impl Future for IsolateWorker {
    type Output = Result<(), ErrBox>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let inner = self.get_mut();
        let waker = AtomicWaker::new();
        waker.register(cx.waker());
        match inner.isolate.try_lock() {
            Ok(mut isolate) => isolate.poll_unpin(cx),
            Err(_) => {
                waker.wake();
                Poll::Pending
            }
        }
    }
}

pub fn op_isolate_is_complete(
    args: Value,
    _zero_copy: Option<PinnedBuf>,
) -> Result<JsonOp, ErrBox> {
    let args: IsolateIsCompleteOptions = serde_json::from_value(args)?;

    let lock = ISOLATE_MAP.read().unwrap();
    let isolate = lock.get(&args.rid).unwrap().clone();
    drop(lock);

    let fut = IsolateWorker { isolate }.map_ok(|_| json!({}));

    Ok(JsonOp::Async(fut.boxed()))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IsolateRegisterOpOptions {
    pub rid: u32,
    pub dispatcher_rid: u32,
    pub name: String,
}

pub fn op_isolate_register_op(
    args: Value,
    _zero_copy: Option<PinnedBuf>,
) -> Result<JsonOp, ErrBox> {
    let args: IsolateRegisterOpOptions = serde_json::from_value(args)?;

    let lock = ISOLATE_MAP.read().unwrap();
    let isolate = lock.get(&args.rid).unwrap();
    let dispatcher = get_dispatcher(args.dispatcher_rid);
    let isolate_lock = isolate.try_lock().unwrap();
    let op_id = isolate_lock.register_op(&args.name, move |data, zero_copy| {
        dispatcher.dispatch(data, zero_copy)
    });
    Ok(JsonOp::Sync(json!({ "opId": op_id })))
}

#[derive(Deserialize)]
struct IsolateExecuteOptions {
    pub rid: u32,
    pub filename: String,
    pub source: String,
}

pub fn op_isolate_execute(args: Value, _zero_copy: Option<PinnedBuf>) -> Result<JsonOp, ErrBox> {
    let args: IsolateExecuteOptions = serde_json::from_value(args)?;

    let lock = ISOLATE_MAP.read().unwrap();
    let isolate = lock.get(&args.rid).unwrap().clone();
    drop(lock);

    let fut = async move {
        let mut isolate_lock = isolate.try_lock().unwrap();
        isolate_lock.execute(&args.filename, &args.source)
    }
    .map_ok(|_| json!({}))
    .boxed();
    let pool = futures::executor::ThreadPool::new().unwrap();
    let fut_handle = pool.spawn_with_handle(fut).unwrap();

    Ok(JsonOp::Async(fut_handle.boxed()))
}

#[derive(Deserialize)]
struct IsolateExecuteModuleOptions {
    pub rid: u32,
    pub module_specifier: String,
}

pub fn op_isolate_execute_module(
    args: Value,
    _zero_copy: Option<PinnedBuf>,
) -> Result<JsonOp, ErrBox> {
    let args: IsolateExecuteModuleOptions = serde_json::from_value(args)?;

    let lock = ISOLATE_MAP.read().unwrap();
    let isolate = lock.get(&args.rid).unwrap().clone();
    drop(lock);

    let fut = async move {
        let mut i = isolate.try_lock().unwrap();
        let id = i.load_module(&args.module_specifier, None).await?;
        let result = i.mod_evaluate(id);
        result
    }
    .map_ok(|_| json!({}))
    .boxed();
    let pool = futures::executor::ThreadPool::new().unwrap();
    let fut_handle = pool.spawn_with_handle(fut).unwrap();

    Ok(JsonOp::Async(fut_handle.boxed()))
}

#[derive(Deserialize)]
struct IsolateSnapshotOptions {
    pub rid: u32,
}

pub fn op_isolate_snapshot(args: Value, _zero_copy: Option<PinnedBuf>) -> Result<JsonOp, ErrBox> {
    let args: IsolateSnapshotOptions = serde_json::from_value(args)?;

    let lock = ISOLATE_MAP.read().unwrap();
    let isolate = lock.get(&args.rid).unwrap().clone();
    drop(lock);

    let mut i = isolate.try_lock().unwrap();
    let snapshot = i.snapshot()?;
    let snapshot_buf: Buf = (**snapshot).into();

    Ok(JsonOp::Sync(json!(ResourceIdResponse {
        rid: crate::snapshots::new_snapshot(snapshot_buf.into()),
    })))
}
