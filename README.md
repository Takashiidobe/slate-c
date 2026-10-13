# Slate C Compiler

Compile C to x86_64 or aarch64 machine code with Slate and Cranelift. Inspect
CLIF, assembly, and binary output, then run your program in the browser.

## Run in the browser

Follow the [browser setup instructions](www/README.md) to prepare the headers
and emulator dependencies, then build and serve the compiler:

```sh
rustup target add wasm32-wasip1
./www/build.sh
python3 www/serve.py
```

Open <http://127.0.0.1:8000>, choose a target, and click **Compile and run** or
press Ctrl/Cmd+Enter. Start with:

```c
int main(void) {
    return 42;
}
```

Compilation and execution happen locally in your browser. The execution panel
shows `main`'s return value. Click **Stop** to cancel a run.

## Inspect output

Choose **Cranelift IR**, **Assembly**, **Machine code (hex)**, or **ELF object
(hex)**. For x86_64 assembly, choose Intel or AT&T syntax. Assembly disassembles
machine code before relocation, so symbol addresses can be placeholders.

Click **Download object** to save the compiled ELF object.

## Configure and share

Enter flags separated by commas. Quote values that contain commas.

- **C compiler flags** configure the dialect and preprocessor. Default:
  `-std=c17, -ffreestanding`.
- **Cranelift flags** configure optimization and code generation. Default:
  `opt_level=speed`. Expand **Cranelift settings** to browse the controls.

Use Cranelift optimization settings instead of C `-O` flags.

Your source and settings are saved in local storage. Click **Share** to update
and copy the current URL with your source and settings. Opening a shared link
uses its settings in preference to locally saved values.

## Resolve compilation and execution errors

Browser execution requires `int main(void)`. You can use scalar functions,
locals, globals, arithmetic, conditionals, loops, pointers, and strings. Other
constructs may produce unsupported-feature errors.

Execution provides no operating system or libc. Calls such as `printf` and
`malloc` fail with unresolved-symbol errors; including a header only supplies
declarations. Runs stop after five seconds, and the guest image must fit within
4 MiB.

If emulation reports missing cross-origin isolation, serve the site with
`www/serve.py`. See [browser hosting](www/README.md#hosting) for the required
headers and browser capabilities.

## Run natively

The native commands currently support constant-return programs:

```sh
cargo run --bin slate-c -- run tests/fixtures/return42.c
cargo run --bin slate-c -- object tests/fixtures/return42.c target/return42.o
cc target/return42.o -o target/return42
```

The first command prints `42`. The linked executable exits with status `42`.
