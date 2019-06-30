use deno::CoreOp;
use deno::PinnedBuf;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate deno;

mod errors;
mod isolate;
mod msg;
mod util;

declare_plugin_op!(new_startup_data, isolate::op_new_startup_data);
declare_plugin_op!(new_isolate, isolate::op_new_isolate);