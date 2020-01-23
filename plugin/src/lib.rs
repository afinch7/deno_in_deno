use deno_core::*;
use deno_dispatch_json::json_op;

#[macro_use]
extern crate lazy_static;

mod dispatch;
mod isolate;
mod modules;
mod msg;

pub use dispatch::Dispatcher;
pub use dispatch::GetDispatcherAccessor;
pub use dispatch::InsertDispatcherAccessor;

pub fn init(cx: &mut dyn PluginInitContext) {
    // Dispatch ops
    cx.register_op(
        "getDispatcherAccessorPtrs",
        json_op(Box::new(dispatch::op_get_dispatcher_accessor_ptrs)),
    );
    cx.register_op(
        "newStdDispatcher",
        json_op(Box::new(dispatch::op_new_std_dispatcher)),
    );
    cx.register_op(
        "stdDispatcherWaitForDispatch",
        json_op(Box::new(dispatch::op_std_dispatcher_wait_for_dispatch)),
    );
    cx.register_op(
        "stdDispatcherRespond",
        json_op(Box::new(dispatch::op_std_dispatcher_respond)),
    );

    // Isolate ops
    cx.register_op(
        "newStartupData",
        json_op(Box::new(isolate::op_new_startup_data)),
    );
    cx.register_op("newIsolate", json_op(Box::new(isolate::op_new_isolate)));
    cx.register_op(
        "isolateIsComplete",
        json_op(Box::new(isolate::op_isolate_is_complete)),
    );
    cx.register_op(
        "isolateRegisterOp",
        json_op(Box::new(isolate::op_isolate_register_op)),
    );
    cx.register_op(
        "isolateExecute",
        json_op(Box::new(isolate::op_isolate_execute)),
    );
    cx.register_op(
        "isolateExecuteModule",
        json_op(Box::new(isolate::op_isolate_execute_module)),
    );

    // Module ops
    cx.register_op(
        "newStdLoader",
        json_op(Box::new(modules::op_new_std_loader)),
    );
    cx.register_op(
        "stdLoaderAwaitResolve",
        json_op(Box::new(modules::op_std_loader_await_resolve)),
    );
    cx.register_op(
        "stdLoaderRespondResolve",
        json_op(Box::new(modules::op_std_loader_respond_resolve)),
    );
    cx.register_op(
        "stdLoaderAwaitLoad",
        json_op(Box::new(modules::op_std_loader_await_load)),
    );
    cx.register_op(
        "stdLoaderRespondLoad",
        json_op(Box::new(modules::op_std_loader_respond_load)),
    );
}

init_fn!(init);
