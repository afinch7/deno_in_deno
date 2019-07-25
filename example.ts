import { Isolate, StdDispatcher, ModuleStore, StdLoader, getDispatcherAccessors } from "./plugin/mod.ts";
import { CustomDispatcher } from "./test_dispatcher/mod.ts";

const textEncoder = new TextEncoder();

const textDecoder = new TextDecoder();

const isolate = new Isolate();

const source = `

const data = new Uint8Array([116, 101, 115, 116]);

async function main() {
    const response = Deno.core.dispatch(data);
    Deno.core.print(\`GUEST RUNTIME RECIEVED RESPONSE \${response} \n\`);
}

main();
`;

/*
const dispatcher = new StdDispatcher();

dispatcher.ondispatch = (data: Uint8Array, zero_copy?: Uint8Array): Uint8Array => {
    console.log(`HOST RUNTIME RECIEVED DISPATCH ${textDecoder.decode(data)}`);
    const response = textEncoder.encode("Hello World!");
    console.log(`HOST RUNTIME SENDING RESPONSE ${response}`);
    return response;
};
*/

const dispatcher = new CustomDispatcher();

isolate.setDispatcher(dispatcher);

const moduleStore = new ModuleStore();

const loader = new StdLoader(
    (specifier, referrer, isRoot) => {
        console.log(`RESOLVE REQUEST ${specifier} ${referrer} ${isRoot}`);
        return "file:///testmod.js";
    },
    (moduleSpecifier) => {
        console.log(`LOAD REQUEST ${moduleSpecifier}`);
        return {
            module_name: moduleSpecifier,
            code: source,
        };
    },
);

async function main() {
    console.log("PRE EXECUTE");
    await isolate.executeModule(
        "test",
        loader,
        moduleStore,
    );
    Deno.exit()
}

main();

console.log(getDispatcherAccessors());
