use crate::msg::ResourceId;
use crate::msg::ResourceIdResponse;
use deno_core::*;
use deno_dispatch_json::JsonOp;
use serde::Deserialize;
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::Mutex;

lazy_static! {
    static ref NEXT_SNAPSHOT_ID: AtomicU32 = AtomicU32::new(1);
    static ref SNAPSHOT_MAP: Mutex<HashMap<u32, Arc<Buf>>> = Mutex::new(HashMap::new());
}

pub fn new_snapshot(snapshot: Buf) -> ResourceId {
    let snaptshot_id = NEXT_SNAPSHOT_ID.fetch_add(1, Ordering::SeqCst);
    let mut lock = SNAPSHOT_MAP.try_lock().unwrap();
    lock.insert(snaptshot_id, Arc::new(snapshot.into()));
    snaptshot_id
}

pub fn snapshot_as_startup_data(snapshot_rid: ResourceId) -> StartupData<'static> {
    let lock = SNAPSHOT_MAP.try_lock().unwrap();
    let data = lock.get(&snapshot_rid).unwrap().clone();
    let data_ptr: *const u8 = data[..].as_ptr();
    let startup_data = unsafe { std::slice::from_raw_parts(data_ptr, data.len()) };
    let owned_startup_data = StartupData::Snapshot(startup_data);
    owned_startup_data
}

pub fn op_new_snapshot(_args: Value, zero_copy: Option<ZeroCopyBuf>) -> Result<JsonOp, ErrBox> {
    assert!(zero_copy.is_some());
    let startup_data = zero_copy.unwrap().to_vec();
    Ok(JsonOp::Sync(json!(ResourceIdResponse {
        rid: new_snapshot(startup_data.into()),
    })))
}

#[derive(Deserialize)]
struct SnapshotReadArgs {
    pub rid: u32,
}

pub fn op_snapshot_read(args: Value, _zero_copy: Option<ZeroCopyBuf>) -> Result<JsonOp, ErrBox> {
    let args: SnapshotReadArgs = serde_json::from_value(args)?;

    let lock = SNAPSHOT_MAP.try_lock().unwrap();
    let data = lock.get(&args.rid).unwrap().clone();

    Ok(JsonOp::Sync(json!({"data": data[..]})))
}
