import { Isolate, StandardDispatcher } from "./plugin/mod.ts";
import { join, dirname } from "https://deno.land/std/fs/path/mod.ts";

const { readFileSync } = Deno;

const textEncoder = new TextEncoder();

const isolate = new Isolate();

const url = new URL(import.meta.url);

const test_script_path = join(dirname(url.pathname), "test_script.js");

const source = new TextDecoder().decode(readFileSync(test_script_path));

const testDispatcher = new StandardDispatcher();

isolate.setDispatcher(testDispatcher);

testDispatcher.ondispatch = (data: Uint8Array, zero_copy?: Uint8Array): Uint8Array => {
    console.log(data);
    const response = textEncoder.encode("Hello World!");
    console.log(`Response: ${response}`);
    return response;
};

async function main() {
    await isolate.execute(
        source,
    );
    console.log("INIT EXECUTE COMPLETE");
    await isolate.execute(
        "main()",
    );
}

main();
