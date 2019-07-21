import { Isolate, StandardDispatcher } from "./plugin/mod.ts";

const textEncoder = new TextEncoder();

const textDecoder = new TextDecoder();

const isolate = new Isolate();

const source = `
const data = new Uint8Array([116, 101, 115, 116]);

async function main() {
    while(true) {
       const response = Deno.core.dispatch(data);
       Deno.core.print(\`GUEST RUNTIME RECIEVED RESPONSE \${response} \n\`);
    }
}
`;

const dispatcher = new StandardDispatcher();

isolate.setDispatcher(dispatcher);

dispatcher.ondispatch = (data: Uint8Array, zero_copy?: Uint8Array): Uint8Array => {
    console.log(`HOST RUNTIME RECIEVED DISPATCH ${textDecoder.decode(data)}`);
    const response = textEncoder.encode("Hello World!");
    console.log(`HOST RUNTIME SENDING RESPONSE ${response}`);
    return response;
};

async function main() {
    await isolate.execute(
        source,
    );
    await isolate.execute(
        "main()",
    );
}

main();
