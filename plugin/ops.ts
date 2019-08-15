import { build } from "../../deno_std/cargo/mod.ts";
import { join, dirname } from "https://deno.land/std/fs/path/mod.ts";

const { openPlugin, pluginFilename } = Deno;

const manifest_path = join(dirname(import.meta.url), "Cargo.toml");

/*
const buildResult = build({
  manifest_path
});
*/

// Load from manual build
let url = new URL(import.meta.url);
const path = join(url.pathname, "../../target/debug")
const buildResult = {
  output_root: path,
  artifacts: [
    {
      output_name: "deno_in_deno",
    }
  ]
}

const plugin = openPlugin(
  join(
    buildResult.output_root,
    pluginFilename(buildResult.artifacts[0].output_name)
  )
);

// StandardDispatcher ops
export const getDispatcherAccessorPtrs = plugin.loadOp("get_dispatcher_accessor_ptrs");
export const newStdDispatcher = plugin.loadOp("new_std_dispatcher");
export const stdDispatcherWaitForDispatch = plugin.loadOp("std_dispatcher_wait_for_dispatch");
export const stdDispatcherRespond = plugin.loadOp("std_dispatcher_respond");

// Isolate ops
export const newStartupData = plugin.loadOp("new_startup_data");
export const newIsolate = plugin.loadOp("new_isolate");
export const isolateIsComplete = plugin.loadOp("isolate_is_complete");
export const isolateSetDispatcher = plugin.loadOp("isolate_set_dispatcher");
export const isolateExecute = plugin.loadOp("isolate_execute");
export const isolateExecuteModule = plugin.loadOp("isolate_execute_module");

// Module ops
export const newModuleStore = plugin.loadOp("new_module_store");
export const newStdLoader = plugin.loadOp("new_std_loader");
export const stdLoaderAwaitResolve = plugin.loadOp("std_loader_await_resolve");
export const stdLoaderRespondResolve = plugin.loadOp("std_loader_respond_resolve");
export const stdLoaderAwaitLoad = plugin.loadOp("std_loader_await_load");
export const stdLoaderRespondLoad = plugin.loadOp("std_loader_respond_load");