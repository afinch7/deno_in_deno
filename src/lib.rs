use deno::CoreOp;
use deno::PinnedBuf;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate deno;

mod dispatch;
mod errors;
mod isolate;
mod msg;
mod util;

pub use dispatch::insert_dispatcher;
pub use dispatch::get_dispatcher;

// Dispatch ops
declare_plugin_op!(new_standard_dispatcher, dispatch::op_new_standard_dispatcher);
declare_plugin_op!(standard_dispatcher_wait_for_dispatch, dispatch::op_standard_dispatcher_wait_for_dispatch);
declare_plugin_op!(standard_dispatcher_respond, dispatch::op_standard_dispatcher_respond);

// Isolate ops
declare_plugin_op!(new_startup_data, isolate::op_new_startup_data);
declare_plugin_op!(new_isolate, isolate::op_new_isolate);
