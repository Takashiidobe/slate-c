# Slate C Compiler

Compile C to x86_64 or AArch64 machine code with Slate and Cranelift. Inspect
the compiler's intermediate forms and machine output, then run your program in
the browser.

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
shows the process exit status and program output. Click **Stop** to cancel a
run.

## Inspect output

Choose the **Cranelift IR**, **Assembly**, **ELF**, **Slate AST**, **Slate IR**,
or **Tokens** tab. In Assembly, **Show bytes** displays instruction encodings
beside the disassembly. In ELF, **Show meaning** decodes header and section
fields. Assembly shows code before relocation, so symbol addresses can be
placeholders.

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

Programs link statically with musl. x86_64 executes with Blink, and AArch64
executes with nixvm without booting Linux. Formatted output, allocation,
strings, and file I/O depend on the selected backend's supported syscalls.
Threads, process creation, sockets, and interactive input are unavailable.

Runs stop after five seconds. Undefined symbols produce linker errors; other
unsupported language features produce compiler errors. See the
[browser instructions](www/README.md) for build requirements and runtime limits.

## Run natively

The native commands currently support constant-return programs:

```sh
cargo run --bin slate-c -- run tests/fixtures/return42.c
cargo run --bin slate-c -- object tests/fixtures/return42.c target/return42.o
cc target/return42.o -o target/return42
```

The first command prints `42`. The linked executable exits with status `42`.
