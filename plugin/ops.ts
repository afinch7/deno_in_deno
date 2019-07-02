import { build } from "../../deno_std/cargo/mod.ts";
import { join, dirname } from "https://deno.land/std/fs/path/mod.ts";

const { openPlugin, pluginFilename } = Deno;

const manifest_path = join(dirname(import.meta.url), "Cargo.toml");

const buildResult = build({
  manifest_path
});

/*
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
*/

// We could also search through the artifacts list here to find something more specific if we wanted.
const plugin = openPlugin(
  join(
    buildResult.output_root,
    pluginFilename(buildResult.artifacts[0].output_name)
  )
);

// StandardDispatcher ops
export const newStandardDispatcher = plugin.loadOp("new_standard_dispatcher");
export const standardDispatcherWaitForDispatch = plugin.loadOp("standard_dispatcher_wait_for_dispatch");
export const standardDispatcherRespond = plugin.loadOp("standard_dispatcher_respond");

// Isolate ops
export const newStartupData = plugin.loadOp("new_startup_data");
export const newIsolate = plugin.loadOp("new_isolate");
export const isolateIsComplete = plugin.loadOp("isolate_is_complete");
export const isolateSetDispatcher = plugin.loadOp("isolate_set_dispatcher");
export const isolateExecute = plugin.loadOp("isolate_execute");