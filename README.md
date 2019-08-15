# Deno in Deno
Create custom deno based runtimes in a deno runtime.

This project is still extremely wip, but it is functional trust me. Steps that might let you get this working:

1. Clone and build this very specific branch of deno https://github.com/afinch7/deno/tree/native_bindings_rust_futures_api

2. Adjust dependency paths for `deno` in cargo packages to correctly point to `//core` in the above cloned repository.

2. `cargo build`

3. `deno run example.ts`

If you manage to do everything just right you should have a functional demo of deno being embeded in deno.
This may still refuse to function even if you do everything right. Keep in mind this project basically ammounts to a demo
built on top of several additional layers of wip(denoland/deno#2385, denoland/deno#2612, futures 3.0, tokio 2.0, etc).
