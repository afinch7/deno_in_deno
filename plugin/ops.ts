import { join, pluginFilename, DispatchJsonPluginOp } from "./deps.ts";

const { openPlugin } = Deno;

// const manifest_path = join(dirname(import.meta.url), "Cargo.toml");

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
export const getDispatcherAccessorPtrs = new DispatchJsonPluginOp(plugin.ops.getDispatcherAccessorPtrs);
export const newStdDispatcher = new DispatchJsonPluginOp(plugin.ops.newStdDispatcher);
export const stdDispatcherWaitForDispatch = new DispatchJsonPluginOp(plugin.ops.stdDispatcherWaitForDispatch);
export const stdDispatcherRespond = new DispatchJsonPluginOp(plugin.ops.stdDispatcherRespond);

// Isolate ops
export const newIsolate = new DispatchJsonPluginOp(plugin.ops.newIsolate);
export const isolateIsComplete = new DispatchJsonPluginOp(plugin.ops.isolateIsComplete);
export const isolateRegisterOp = new DispatchJsonPluginOp(plugin.ops.isolateRegisterOp);
export const isolateExecute = new DispatchJsonPluginOp(plugin.ops.isolateExecute);
export const isolateExecuteModule = new DispatchJsonPluginOp(plugin.ops.isolateExecuteModule);
export const isolateSnapshot = new DispatchJsonPluginOp(plugin.ops.isolateSnapshot);

// Module ops
export const newStdLoader = new DispatchJsonPluginOp(plugin.ops.newStdLoader);
export const stdLoaderAwaitResolve = new DispatchJsonPluginOp(plugin.ops.stdLoaderAwaitResolve);
export const stdLoaderRespondResolve = new DispatchJsonPluginOp(plugin.ops.stdLoaderRespondResolve);
export const stdLoaderAwaitLoad = new DispatchJsonPluginOp(plugin.ops.stdLoaderAwaitLoad);
export const stdLoaderRespondLoad = new DispatchJsonPluginOp(plugin.ops.stdLoaderRespondLoad);

// Snapshot ops
export const newSnapshot = new DispatchJsonPluginOp(plugin.ops.newSnapshot);
export const snapshotRead = new DispatchJsonPluginOp(plugin.ops.snapshotRead);