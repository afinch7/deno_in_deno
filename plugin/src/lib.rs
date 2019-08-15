#![feature(async_await, await_macro)]
use deno::CoreOp;
use deno::PinnedBuf;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate deno;

mod dispatch;
mod errors;
mod isolate;
mod modules;
mod msg;
mod tokio_util;
mod util;

pub use dispatch::GetDispatcherAccessor;
pub use dispatch::InsertDispatcherAccessor;
pub use dispatch::Dispatcher;

// Dispatch ops
declare_plugin_op!(get_dispatcher_accessor_ptrs, dispatch::op_get_dispatcher_accessor_ptrs);
declare_plugin_op!(new_std_dispatcher, dispatch::op_new_std_dispatcher);
declare_plugin_op!(std_dispatcher_wait_for_dispatch, dispatch::op_std_dispatcher_wait_for_dispatch);
declare_plugin_op!(std_dispatcher_respond, dispatch::op_std_dispatcher_respond);

// Isolate ops
declare_plugin_op!(new_startup_data, isolate::op_new_startup_data);
declare_plugin_op!(new_isolate, isolate::op_new_isolate);
declare_plugin_op!(isolate_is_complete, isolate::op_isolate_is_complete);
declare_plugin_op!(isolate_set_dispatcher, isolate::op_isolate_set_dispatcher);
declare_plugin_op!(isolate_execute, isolate::op_isolate_execute);
declare_plugin_op!(isolate_execute_module, isolate::op_isolate_execute_module);

// Module ops
declare_plugin_op!(new_module_store, modules::op_new_module_store);
declare_plugin_op!(new_std_loader, modules::op_new_std_loader);
declare_plugin_op!(std_loader_await_resolve, modules::op_std_loader_await_resolve);
declare_plugin_op!(std_loader_respond_resolve, modules::op_std_loader_respond_resolve);
declare_plugin_op!(std_loader_await_load, modules::op_std_loader_await_load);
declare_plugin_op!(std_loader_respond_load, modules::op_std_loader_respond_load);