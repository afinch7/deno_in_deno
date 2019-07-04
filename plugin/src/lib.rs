#![feature(async_await, await_macro, futures_api)]
use deno::plugins::PluginOp;
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
pub use dispatch::Dispatcher;

// Dispatch ops
declare_plugin_op!(new_standard_dispatcher, dispatch::op_new_standard_dispatcher);
declare_plugin_op!(standard_dispatcher_wait_for_dispatch, dispatch::op_standard_dispatcher_wait_for_dispatch);
declare_plugin_op!(standard_dispatcher_respond, dispatch::op_standard_dispatcher_respond);

// Isolate ops
declare_plugin_op!(new_startup_data, isolate::op_new_startup_data);
declare_plugin_op!(new_isolate, isolate::op_new_isolate);
declare_plugin_op!(isolate_is_complete, isolate::op_isolate_is_complete);
declare_plugin_op!(isolate_set_dispatcher, isolate::op_isolate_set_dispatcher);
declare_plugin_op!(isolate_execute, isolate::op_isolate_execute);
