Deno.core.print('Hello World')

const data = new Uint8Array([116, 101, 115, 116]);

while(true) {
    const response = Deno.core.dispatch(data);
    Deno.core.print(response);
}




