Deno.core.print('Hello World\n')

const data = new Uint8Array([116, 101, 115, 116]);

async function main() {
    while(true) {
       const response = Deno.core.dispatch(data);
       Deno.core.print(response);
    }
}

Deno.core.print("INIT COMPLETE\n");
