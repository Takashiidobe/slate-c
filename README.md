# slate-c

Compile C through Slate's typed IR and Cranelift. The browser emits x86-64 or
AArch64 machine code, shows CLIF and assembly, and calls `main` directly under
bare-metal QEMU. Compilation and execution stay in the browser.

## Browser

```sh
./www/build.sh
python3 www/serve.py
```

Open <http://127.0.0.1:8000>. Choose a target and example, then use **Compile and
run** or Ctrl/Cmd+Enter. Source and flag edits refresh compiler output. Headers
and the selected emulator load lazily and are cached during the session.

C compiler flags configure Slate's dialect and preprocessor. Cranelift flags
configure optimization and the target backend; expand **Cranelift settings**
for the available controls. Use `opt_level=speed` instead of C `-O` flags.

The page reports `main`'s signed integer return value, faults, or timeouts.
**Stop** cancels execution. Download the compiled ELF object to link elsewhere.
Execution does not provide an OS or libc; unresolved external symbols such as
`puts` or `malloc` are rejected. Header declarations do not supply libraries.

The build needs Rust's `wasm32-wasip1` target, Python, and sibling checkouts of
`../slate` and `../clif-explorer` with the explorer's QEMU and Capstone builds
already present. Those JS/Wasm artifacts are copied directly without rebuilding.
See [www/README.md](www/README.md) for header paths and hosting.

## Supported C

Execution requires `int main(void)`. The browser backend supports scalar
functions, locals, scalar globals, integer/f32/f64 arithmetic, conditionals,
structured loops, pointers, and strings. Examples cover helpers and globals,
loops, string data, and floating-point calls.

Unsupported features produce errors. Variadic calls/definitions, aggregate
function ABI passes, extended floats, atomic/volatile access, inline assembly,
switch/goto, TLS, static locals, and some initializer forms are not supported.
The guest image is limited to 4 MiB and execution to five seconds. This is an
initial C backend, not a complete C compiler.

## Native

The native JIT and object commands retain the initial constant-return subset:

```sh
cargo run --bin slate-c -- run tests/fixtures/return42.c
cargo run --bin slate-c -- object tests/fixtures/return42.c target/return42.o
cc target/return42.o -o target/return42
```

The JIT prints `42`; the linked executable exits with status `42`. Native flag
controls and the broader lowering can be added later.

## Development

```sh
cargo test --all-features
cargo clippy --all-features --all-targets -- -D warnings
```

The implementation plan and progress are in [HANDOFF.md](HANDOFF.md).
Dependencies no longer include clif2wasm or Waffle. Wasm hosts the compiler and
emulator; generated programs are native ELF objects.
