import { build } from "../../deno_std/cargo/mod.ts";
import { join, dirname } from "https://deno.land/std/fs/path/mod.ts";
import { Dispatcher } from "../plugin/mod.ts";

const { openPlugin, pluginFilename } = Deno;

const manifest_path = join(dirname(import.meta.url), "Cargo.toml");
const buildResult = build({
  manifest_path,
});
// We could also search through the artifacts list here to find something more specific if we wanted.
const plugin = openPlugin(
  join(
    buildResult.output_root,
    pluginFilename(buildResult.artifacts[0].output_name)
  )
);

const textEncoder = new TextEncoder();

function encodeMessage(message: any): Uint8Array {
    return textEncoder.encode(JSON.stringify(message));
}

const textDecoder = new TextDecoder();

function decodeMessage<D = any>(message: Uint8Array): any {
    return JSON.parse(textDecoder.decode(message));
}

type OpResponse = undefined | Uint8Array;

function wrapSyncOp(response: Promise<OpResponse> | OpResponse): Uint8Array {
    if (response instanceof Uint8Array) {
        return response;
    } else {
        throw new Error(`Unexpected response type for sync op ${typeof response}`);
    }
}

const newTestDispatcher = plugin.loadOp("new_test_dispatcher");

export class TestDispatcher implements Dispatcher {

    private readonly rid_: number;

    constructor() {
        this.rid_ = decodeMessage(
            wrapSyncOp(
                newTestDispatcher.dispatch(
                    encodeMessage(""),
                ),
            ),
        );
    }

    get rid(): number {
        return this.rid_;
    }
}

