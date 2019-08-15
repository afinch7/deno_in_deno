#[macro_use]
extern crate deno;

use deno::CoreOp;
use deno::Op;
use deno::PinnedBuf;
use deno_in_deno::Dispatcher;
use deno_in_deno::InsertDispatcherAccessor;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;

struct CustomDispatcher;

impl Dispatcher for CustomDispatcher {
    fn dispatch(&self, data: &[u8], _zero_copy: Option<PinnedBuf>) -> CoreOp {
        dbg!(data);
        let result = b"test1234";
        Op::Sync(result[..].into())
    }
}

#[derive(Deserialize)]
struct NewCustomDispatcherOptions {
    pub getDispatcher: usize,
    pub insertDispatcher: usize,
}

#[derive(Serialize)]
struct NewCustomDispatcherResponse {
    pub rid: u32,
}

pub fn op_new_custom_dispatcher(
    data: &[u8],
    _zero_copy: Option<PinnedBuf>,
) -> CoreOp {
    let data_str = std::str::from_utf8(&data[..]).unwrap();
    let options: NewCustomDispatcherOptions = serde_json::from_str(data_str).unwrap();
    let insert_dispatcher = unsafe { *(options.insertDispatcher as *const InsertDispatcherAccessor) };
    let dispacher: Arc<Box<dyn Dispatcher>> = Arc::new(Box::new(CustomDispatcher));
    let rid = insert_dispatcher(dispacher);
    let result = NewCustomDispatcherResponse {
        rid,
    };
    let result_json = serde_json::to_string(&result).unwrap();
    Op::Sync(result_json.as_bytes().into())
}

declare_plugin_op!(new_custom_dispatcher, op_new_custom_dispatcher);