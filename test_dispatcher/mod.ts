// import { build } from "../../deno_std/cargo/mod.ts";
import { join, pluginFilename, DispatchJsonPluginOp, getDispatcherAccessors, Dispatcher } from "./deps.ts";

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
      output_name: "test_dispatcher",
    }
  ]
}

const plugin = openPlugin(
  join(
    buildResult.output_root,
    pluginFilename(buildResult.artifacts[0].output_name)
  )
);

const newCustomDispatcher = new DispatchJsonPluginOp(plugin.ops.newCustomDispatcher);

const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder();

type OpResponse = undefined | Uint8Array;
type OpResponseAnySync = Promise<OpResponse> | OpResponse;

export function wrapSyncOp(response: OpResponseAnySync): Uint8Array {
    if (response instanceof Uint8Array) {
        return response;
    } else {
        throw new Error(`Unexpected response type for sync op ${typeof response}`);
    }
}

export class CustomDispatcher implements Dispatcher {

  private readonly rid_: number;

  constructor() {
    const response = newCustomDispatcher.dispatchSync(getDispatcherAccessors());
    this.rid_ = response.rid;
  }

  get rid(): number {
    return this.rid_;
  }

}