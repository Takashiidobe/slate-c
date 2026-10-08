# slate-c

A small C backend consuming Slate’s resolved IR and lowering it to Cranelift IR.
The first supported program is:

```c
int main(void) {
    return 42;
}
```

The same lowering feeds native JIT execution, native object emission, and the
clif2wasm/Waffle bridge for WebAssembly. Both the generated program and the
compiler itself can run in the browser. Unsupported IR produces an error.
Currently only one parameterless `main` returning a signed 32-bit integer
constant is supported; locals, calls, globals, and other statements are not.

## Native

```sh
cargo run -- run tests/fixtures/return42.c
cargo run -- object tests/fixtures/return42.c target/return42.o
cc target/return42.o -o target/return42
./target/return42
```

The JIT command prints `42`. The linked executable exits with status `42`.
To emit Wasm from the native CLI:

```sh
cargo run -- wasm tests/fixtures/return42.c target/return42.wasm
```

## Browser

```sh
./www/build.sh
python3 -m http.server --directory www/dist 8000
```

Open <http://localhost:8000>. The UI follows Slate’s compiler layout, showing
C beside the generated CLIF, with an execution result and a Wasm download.
It compiles in a Web Worker and executes the generated Wasm locally.
The build requires `wasm-pack` and the `wasm32-unknown-unknown` Rust target.
See [www/README.md](www/README.md).

## Validation

```sh
cargo test
cargo clippy --all-targets -- -D warnings
node www/smoke.mjs
```

Tests require Node.js for Wasm execution. The smoke test requires a browser
build; it executes the Wasm-hosted compiler, checks the emitted CLIF, and
executes generated programs returning 42 and 17. Invalid and unsupported
source is rejected.

## Architecture and current limits

```text
C -> Slate parse / semantic analysis -> Slate IR -> CLIF
                                                   -> Cranelift JIT / object
                                                   -> clif2wasm -> Waffle -> Wasm
```

`parse` exposes Slate’s resolved `ir::Module`, and `lower` consumes that IR
using Cranelift’s `FunctionBuilder`. CLIF is verified before emission.
Native signatures use the host calling convention. The initial Wasm path
passes functions directly to clif2wasm; it does not need a native ISA or a
native compilation pass. The bridge’s optional `WasmModule` seam can be used
when function/data declarations are added.

Cranelift is pinned to 0.130.0 to match the vendored
[clif2wasm prototype](vendor/clif2wasm/UPSTREAM.md). Slate 0.1.0 is also
[vendored](vendor/slate-parser/UPSTREAM.md) with a browser clock patch:
preprocessor initialization uses JavaScript `Date.now()` on the browser target
instead of Rust’s unsupported `SystemTime::now()`.

Slate 0.1.0 has no wasm32 C target profile. For this initial integer-only
subset, Wasm compilation temporarily uses its i686 profile. Target macros
therefore describe i686 Linux, and this is not a general wasm32 C frontend.
A real wasm32 profile is needed before supporting target-dependent C,
pointers, layouts, or aggregate ABIs. Native compilation uses the host’s
actual Slate profile. Broader clif2wasm coverage also needs validation; its
atomics currently assume single-threaded execution.
