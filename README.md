# dim-lo-process

DIM's Loadout Optimizer, librarified with WASM as a primary compilation target.

Requires nightly Rust to be built.

`cargo compile` generates a wasm blob in `target\wasm32-unknown-unknown\wasm\lo_web.opt.wasm`. Requires wasm-opt from the [Binaryen](https://github.com/WebAssembly/binaryen) toolchain.

Nightly Rust is needed because we build the standard library instead of linking the pre-shipped one (`-Zbuild-std`).
(Actually, we use no_std for the core and wasm library, alloc is sufficient.)
This enables massive code size savings because the shipped standard library is built with panic=unwind, i.e. stack unwinding,
tons of custom panic messages, format runtimes etc. panic=abort can save us up to 140kB.
