use deno::Buf;
use deno::Op;
use deno::CoreOp;
use deno::PinnedBuf;
use deno_in_deno::insert_dispatcher;
use deno_in_deno::Dispatcher;
use std::sync::Arc;
use deno::plugins::PluginOp;

#[macro_use]
extern crate deno;

struct TestDispatcher {
}

impl TestDispatcher {
    pub fn new() -> Self {
        Self {}
    }
}

impl Dispatcher for TestDispatcher {
    fn dispatch(&self, data: &[u8], zero_copy: Option<PinnedBuf>) -> CoreOp {
        if let Some(buf) = zero_copy {
            let data_str = std::str::from_utf8(&data[..]).unwrap();
            let buf_str = std::str::from_utf8(&buf[..]).unwrap();
            println!(
                "Hello from native bindings. data: {} | zero_copy: {}",
                data_str, buf_str
            );
        }
        let result = b"test";
        let result_box: Buf = Box::new(*result);
        Op::Sync(result_box)
    }
}

pub fn op_new_test_dispatcher(
    _data: &[u8],
    _zero_copy: Option<PinnedBuf>,
) -> PluginOp {
    let dispatcher = TestDispatcher::new();
    let rid = insert_dispatcher(Arc::new(Box::new(dispatcher)));

    let result_json = serde_json::to_string(&rid).unwrap();
    PluginOp::Sync(result_json.as_bytes().into())
}

declare_plugin_op!(new_test_dispatcher, op_new_test_dispatcher);