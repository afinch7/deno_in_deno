import {
  Isolate,
  StdDispatcher,
  StdLoader,
} from "./plugin/mod.ts";
import { CustomDispatcher } from "./test_dispatcher/mod.ts";

const textEncoder = new TextEncoder();

const textDecoder = new TextDecoder();

const source = `
function main() {
  const data = new Uint8Array([116, 101, 115, 116]);

  async function callOp(opId) {
    Deno.core.print(\`GUEST RUNTIME CALLING OP \${opId} \n\`);
    const response = Deno.core.dispatch(opId, data);
    Deno.core.print(\`GUEST RUNTIME RECIEVED RESPONSE \${response} \n\`);
  }

  let ops = Deno.core.ops();
  let testOpId = ops.testOp;
  let testOpJsId = ops.testOpJs;
  callOp(testOpId);
  callOp(testOpJsId);
}
`;

const loader = new StdLoader(
  (specifier, referrer, isRoot) => {
    console.log(`RESOLVE REQUEST ${specifier} ${referrer} ${isRoot}`);
    return "file:///testmod.js";
  },
  moduleSpecifier => {
    console.log(`LOAD REQUEST ${moduleSpecifier}`);
    return {
      module_name: moduleSpecifier,
      code: source
    };
  }
);

const isolate = new Isolate(loader, {
  will_snapshot: true
});

const dispatcher = new StdDispatcher();

dispatcher.ondispatch = (
  data: Uint8Array,
  zero_copy?: Uint8Array
): Uint8Array => {
  console.log(`HOST RUNTIME RECIEVED DISPATCH ${textDecoder.decode(data)}`);
  const response = textEncoder.encode("Hello World!");
  console.log(`HOST RUNTIME SENDING RESPONSE ${response}`);
  return response;
};

const customDispatcher = new CustomDispatcher();

async function main() {
  console.log("PRE EXECUTE");
  await isolate.execute(source);
  const snapshot = isolate.snapshot();
  async function loadSnapshotAndExecute() {
    const snapshotIsolate = new Isolate(loader, {
      will_snapshot: false,
      snapshot,
    });
    snapshotIsolate.registerOp("testOp", customDispatcher);
    snapshotIsolate.registerOp("testOpJs", dispatcher);
    await snapshotIsolate.execute("main()");
  }
  for (const x of Array(50).keys()) {
    await loadSnapshotAndExecute();
  }
  Deno.exit();
}

main();