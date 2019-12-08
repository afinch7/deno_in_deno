use deno_core::*;
use deno_dispatch_json::json_op;
use deno_dispatch_json::JsonOp;
use deno_in_deno::Dispatcher;
use deno_in_deno::InsertDispatcherAccessor;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use serde_json::Value;
use std::sync::Arc;

pub fn init(cx: &mut dyn PluginInitContext) {
    cx.register_op(
        "newCustomDispatcher",
        json_op(Box::new(op_new_custom_dispatcher)),
    );
}

init_fn!(init);

struct CustomDispatcher;

impl Dispatcher for CustomDispatcher {
    fn dispatch(&self, data: &[u8], _zero_copy: Option<PinnedBuf>) -> CoreOp {
        dbg!(data);
        let result = b"test1234";
        Op::Sync(result[..].into())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NewCustomDispatcherOptions {
    pub get_dispatcher: usize,
    pub insert_dispatcher: usize,
}

#[derive(Serialize)]
struct NewCustomDispatcherResponse {
    pub rid: u32,
}

pub fn op_new_custom_dispatcher(
    args: Value,
    _zero_copy: Option<PinnedBuf>,
) -> Result<JsonOp, ErrBox> {
    let args: NewCustomDispatcherOptions = serde_json::from_value(args)?;
    let insert_dispatcher = unsafe { *(args.insert_dispatcher as *const InsertDispatcherAccessor) };
    let dispacher: Arc<Box<dyn Dispatcher>> = Arc::new(Box::new(CustomDispatcher));
    let rid = insert_dispatcher(dispacher);
    Ok(JsonOp::Sync(json!(NewCustomDispatcherResponse { rid })))
}
