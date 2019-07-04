use crate::dispatch::get_dispatcher;
use crate::errors::new_error;
use crate::errors::DIDResult;
use crate::util::wrap_op;
use crate::util::serialize_sync_result;
use crate::util::serialize_response;
use crate::util::DIDOp;
use crate::msg::ResourceId;
use crate::msg::ResourceIdResponse;
use crate::msg::EmptyResponse;
use deno::Buf;
use deno::PinnedBuf;
use deno::Isolate;
use deno::StartupData;
use deno::plugins::PluginOp;
use futures::future::Future as NewFuture;
use futures::future::FutureExt;
use futures::task::Poll;
use futures::task::Context;
use futures::channel::oneshot::channel;
use tokio::prelude::future::Future;
use tokio::prelude::Async;
use serde::Deserialize;
use std::collections::HashMap;
use std::cell::RefCell;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use std::sync::RwLock;
use std::sync::Arc;
use std::pin::Pin;

lazy_static! {
    static ref NEXT_STARTUP_DATA_ID: AtomicU32 = AtomicU32::new(1);
    static ref STARTUP_DATA_MAP: Mutex<HashMap<u32, RefCell<Vec<u8>>>> = Mutex::new(HashMap::new());
    static ref NEXT_ISOLATE_ID: AtomicU32 = AtomicU32::new(1);
    static ref ISOLATE_MAP: RwLock<HashMap<u32, Arc<Mutex<Isolate>>>> = RwLock::new(HashMap::new());
}

pub fn op_new_startup_data(
    data: &[u8],
    zero_copy: Option<PinnedBuf>,
) -> PluginOp {
    wrap_op(|_data, zero_copy| {
        assert!(zero_copy.is_some());
        let startup_data = zero_copy.unwrap().to_vec();
        let startup_data_id = NEXT_STARTUP_DATA_ID.fetch_add(1, Ordering::SeqCst);
        let mut lock = STARTUP_DATA_MAP.lock().unwrap();
        lock.insert(startup_data_id, RefCell::new(startup_data));
        serialize_sync_result(ResourceIdResponse {
            rid: startup_data_id,
        })
    }, data, zero_copy)
} 

#[derive(Deserialize)]
struct NewIsolateOptions {
    pub will_snapshot: bool,
    pub startup_data_rid: Option<u32>,
}

pub fn op_new_isolate(
    data: &[u8],
    zero_copy: Option<PinnedBuf>,
) -> PluginOp {
    wrap_op(|data, _zero_copy| {
        let data_str = std::str::from_utf8(&data[..]).unwrap();
        let options: NewIsolateOptions = serde_json::from_str(data_str).unwrap();
        
        match options.startup_data_rid {
            Some(rid) => {
                let lock = STARTUP_DATA_MAP.lock().unwrap();
                let cell = lock.get(&rid).unwrap().borrow();
                let startup_data = StartupData::Snapshot(&cell);
                let isolate_rid = op_new_isolate_inner(startup_data, options.will_snapshot);
                serialize_sync_result(ResourceIdResponse {
                    rid: isolate_rid,
                })
            },
            None => {
                let isolate_rid = op_new_isolate_inner(StartupData::None, options.will_snapshot);
                serialize_sync_result(ResourceIdResponse {
                    rid: isolate_rid,
                })
            },
        }
        // TODO(afinch7) figure out some way to handle startup data.
        
    }, data, zero_copy)
}

fn op_new_isolate_inner(
    startup_data: StartupData,
    will_snapshot: bool,
) -> ResourceId {
    let isolate_rid = NEXT_ISOLATE_ID.fetch_add(1, Ordering::SeqCst);
    let isolate = Isolate::new(startup_data, will_snapshot);
    let mut lock = ISOLATE_MAP.write().unwrap();
    lock.insert(isolate_rid, Arc::new(Mutex::new(isolate)));
    isolate_rid
}

#[derive(Deserialize)]
struct IsolateIsCompleteOptions {
    pub rid: u32,
}

struct IsolateWorker {
    pub rid: u32,
}

impl NewFuture for IsolateWorker {
    type Output = DIDResult<Buf>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Self::Output> {
        let lock = ISOLATE_MAP.read().unwrap();
        let i = lock.get(&self.rid).unwrap();
        let mut isolate = i.lock().unwrap();
        match isolate.poll() {
            Ok(r) => match r {
                Async::Ready(_) => Poll::Ready(Ok(serialize_response(EmptyResponse))),
                Async::NotReady => Poll::Pending,
            },
            Err(err) => Poll::Ready(Err(new_error(&format!("{:#?}", err)))),
        }
    }
}

pub fn op_isolate_is_complete(
    data: &[u8],
    zero_copy: Option<PinnedBuf>,
) -> PluginOp {
    wrap_op(|data, _zero_copy| {
        let data_str = std::str::from_utf8(&data[..]).unwrap();
        let options: IsolateIsCompleteOptions = serde_json::from_str(data_str).unwrap();

        let op = IsolateWorker {
            rid: options.rid,
        };
        Ok(DIDOp::Async(op.boxed()))
    }, data, zero_copy)
}

#[derive(Deserialize)]
struct IsolateSetDispatcherOptions {
    pub rid: u32,
    pub dispatcher_rid: u32,
}

pub fn op_isolate_set_dispatcher(
    data: &[u8],
    zero_copy: Option<PinnedBuf>,
) -> PluginOp {
    wrap_op(|data, _zero_copy| {
        let data_str = std::str::from_utf8(&data[..]).unwrap();
        let options: IsolateSetDispatcherOptions = serde_json::from_str(data_str).unwrap();

        let lock = ISOLATE_MAP.read().unwrap();
        let isolate = lock.get(&options.rid).unwrap();
        let dispatcher = get_dispatcher(options.dispatcher_rid);
        let mut isolate_lock = isolate.lock().unwrap();
        isolate_lock.set_dispatch(move |data, zero_copy| {
            dispatcher.dispatch(data, zero_copy)
        });
        serialize_sync_result(EmptyResponse)
    }, data, zero_copy)
}

#[derive(Deserialize)]
struct IsolateExecuteOptions {
    pub rid: u32,
    pub filename: String,
    pub source: String,
}

pub fn op_isolate_execute(
    data: &[u8],
    zero_copy: Option<PinnedBuf>,
) -> PluginOp {
    wrap_op(|data, _zero_copy| {
        let data_str = std::str::from_utf8(&data[..]).unwrap();
        let options: IsolateExecuteOptions = serde_json::from_str(data_str).unwrap();

        let (sender, receiver) = channel::<DIDResult<Buf>>();
        std::thread::spawn(move || {
            let lock = ISOLATE_MAP.read().unwrap();
            let isolate = lock.get(&options.rid).unwrap();
            let mut isolate_lock = isolate.lock().unwrap();
            let result = match isolate_lock.execute(&options.filename, &options.source) {
                Ok(_) => Ok(serialize_response(EmptyResponse)),
                Err(err) => Err(new_error(&format!("{:#?}", err))),
            };
            dbg!("EXECUTE DONE");
            assert!(sender.send(result).is_ok());
        });

        let op = async {
            receiver.await.unwrap()
        }.boxed();

        Ok(DIDOp::Async(op))
    }, data, zero_copy)
}