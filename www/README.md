# Browser compiler

Build and serve the x86_64 and AArch64 C compiler in your browser:

```sh
rustup target add wasm32-wasip1
./www/build.sh
python3 www/serve.py
```

Open <http://127.0.0.1:8000>. Choose a target, then compile and run your C
program in a fresh browser worker.

## Prepare dependencies

Install Rust, Python, Git, GNU Make, curl, Emscripten (`emcc` and `emar`),
Clang, and `llvm-ar`. Keep sibling checkouts of `../slate` and
`../clif-explorer` with Capstone already built. QEMU is not required.

Install Slate's Clang builtin headers before building. `SLATE_HEADERS` selects
the directory containing `compiler-headers/`; its default is
`~/.local/share/slate`. `CLIF_EXPLORER` selects the explorer checkout; its
default is `../clif-explorer`.

The first build downloads pinned Blink and nixvm sources, musl files, and linker
packages, then builds their browser runtimes. Generated assets are in
`www/dist`; cached sources and packages are in `build`.

The build packages matching musl headers and libraries together. Target headers,
Blink, and linker assets load lazily. No kernel or operating system boots.

## Use the compiler

Select an example or enter a program with `int main(void)`. Press Ctrl/Cmd+Enter
or click **Compile and run**. **Stop** cancels linking or execution. Linking has
a 30-second limit; execution has a five-second limit. Output is limited to 1 MiB
per stream, and each run starts with a fresh virtual filesystem.

Use the output tabs to inspect Cranelift IR, assembly, ELF fields, Slate's AST
S-expression, Slate IR, and source tokens. In Assembly, **Show bytes** puts each
instruction's machine-code bytes beside it. In ELF, **Show meaning** decodes the
header and section fields. Assembly shows code before relocation, so symbol
addresses can be placeholders. **Download object** saves the relocatable ELF.

x86_64 runs with Blink. AArch64 links a static musl executable and runs it with
nixvm. Both paths show the total elapsed time and process output.

Separate flags with commas and quote values containing commas. Source and all
settings persist locally. **Share** updates the current URL and copies a link
containing your source and settings; shared values override local storage.

Musl provides formatted output, allocation, strings, and file I/O. Supported
variadic calls include the printf/scanf families, `dprintf`, `asprintf`, `open`,
`openat`, `fcntl`, and `ioctl`. Other compiler and syscall limitations still
apply. Threads, process creation, sockets, and interactive input are
unavailable.

## Host the site

Serve `www/dist` with a static HTTP server. `www/serve.py` and `dist/_headers`
include isolation headers, but Blink does not require the Memory64 or shared
memory features needed by the former QEMU build. Enable HTTP compression for the
compiler and runtime assets.

Blink and Capstone licenses and runtime package/source references ship in
`dist/licenses`. See [runtime packaging](runtime/README.md) for the linker and
[standalone Blink demo](blink/README.md) to run existing static executables.
