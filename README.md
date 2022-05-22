# dim-lo-process

DIM's Loadout Optimizer, librarified with WASM as a primary compilation target.

Requires nightly Rust to be built.

`cargo compile` generates a wasm blob in `target\wasm32-unknown-unknown\wasm\lo_web.opt.wasm`. Requires wasm-opt from the [Binaryen](https://github.com/WebAssembly/binaryen) toolchain.

Nightly Rust is needed because we build the standard library instead of linking the pre-shipped one (`-Zbuild-std`).
(Actually, we use no_std for the core and wasm library, alloc is sufficient.)
This enables massive code size savings because the shipped standard library is built with panic=unwind, i.e. stack unwinding,
tons of custom panic messages, format runtimes etc. panic=abort can save us up to 140kB.

## Crates

### `lo-core`

Safe Rust `no_std`+`alloc` library that defines the core types and implements the core algorithm.
Structs are `repr(C)` and make size and alignment guarantees so that FFI consumers of the library
can efficiently send data to the algorithm with very few allocations and copies.

### `lo-web`

Library wrapping the core library in a way amenable to WASM FFI. This contains the necessary unsafe
code to retrieve data from FFI and really relies on the host (usually JS) doing the right thing,
getting details wrong on the host side will really mess the algorithm up.

`crate-type = ["cdylib"]` produces a WASM blob. Our custom `wasm` profile uses all available
knobs to bring down the size of the WASM blob by getting rid of all the features we don't
need for correct operation.

### `lo-offline`

The JS side can serialize the inputs of the algorithm to JSON, and the `lo-offline` binary
can load that JSON and run the algorithm "offline", i.e. as a native executable, not embedded
in a browser or any sort of runtime. This allows for far more convenient debugging and profiling,
WASM is really restricted and WASM debugging is mostly hopeless. This can't help with debugging
FFI issues though.

Try `cargo run --release -p lo-offline -- .\export.json`
