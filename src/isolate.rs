use crate::errors::new_error;
use crate::util::wrap_op;
use crate::util::serialize_and_wrap;
use crate::msg::ResourceIdResponse;
use deno::CoreOp;
use deno::PinnedBuf;
use deno::Isolate;
use deno::StartupData;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use std::sync::Arc;

lazy_static! {
    static ref NEXT_STARTUP_DATA_ID: AtomicU32 = AtomicU32::new(0);
    static ref STARTUP_DATA_MAP: Mutex<HashMap<u32, Vec<u8>>> = Mutex::new(HashMap::new());
    static ref NEXT_ISOLATE_ID: AtomicU32 = AtomicU32::new(0);
    static ref ISOLATE_MAP: Mutex<HashMap<u32, Arc<Mutex<Isolate>>>> = Mutex::new(HashMap::new());
}

pub fn op_new_startup_data(
    data: &[u8],
    zero_copy: Option<PinnedBuf>,
) -> CoreOp {
    wrap_op(|_data, zero_copy| {
        match zero_copy {
            Some(buf) => {
                let startup_data = buf.to_vec();
                let startup_data_id = NEXT_STARTUP_DATA_ID.fetch_add(1, Ordering::SeqCst);
                let mut lock = STARTUP_DATA_MAP.lock().unwrap();
                lock.insert(startup_data_id, startup_data);
                serialize_and_wrap(ResourceIdResponse {
                    rid: startup_data_id,
                })
            },
            None => Err(new_error("Unexpected None for zero copy")),
        }
    }, data, zero_copy)
} 

#[derive(Deserialize)]
struct NewIsolateOptions {
    pub will_snapshot: bool,
}

pub fn op_new_isolate(
    data: &[u8],
    zero_copy: Option<PinnedBuf>,
) -> CoreOp {
    wrap_op(|data, _zero_copy| {
        let data_str = std::str::from_utf8(&data[..]).unwrap();
        let options: NewIsolateOptions = serde_json::from_str(data_str).unwrap();
        let isolate_id = NEXT_ISOLATE_ID.fetch_add(1, Ordering::SeqCst);
        // TODO(afinch7) figure out some way to handle startup data.
        let isolate = Isolate::new(StartupData::None, options.will_snapshot);
        let mut lock = ISOLATE_MAP.lock().unwrap();
        lock.insert(isolate_id, Arc::new(Mutex::new(isolate)));
        serialize_and_wrap(ResourceIdResponse {
            rid: isolate_id,
        })
    }, data, zero_copy)
}