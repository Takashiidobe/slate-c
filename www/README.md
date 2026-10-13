# Browser compiler

Build and serve the x86_64 compiler with Blink, musl, and a browser-side linker:

```sh
rustup target add wasm32-wasip1
./www/build.sh
python3 www/serve.py
```

Open <http://127.0.0.1:8000>. Compile and run links your object with musl, runs
it in a fresh worker, and displays output and the process exit status.

## Prepare dependencies

Install Rust, Python, Git, GNU Make, curl, Emscripten (`emcc` and `emar`),
Clang, and `llvm-ar`. Keep sibling checkouts of `../slate` for its browser WASI
shim and `../clif-explorer` with Capstone already built. QEMU is not required.

Install Slate's Clang builtin headers before building. `SLATE_HEADERS` selects
the directory containing `compiler-headers/`; its default is
`~/.local/share/slate`. `CLIF_EXPLORER` selects the explorer checkout; its
default is `../clif-explorer`.

The first build downloads pinned Blink sources and Alpine linker/musl packages,
then builds Blink with Emscripten. Later builds reuse the Blink runtime when its
source revision and compiler settings match. Generated assets are in `www/dist`;
cached sources and packages are in `build`.

The build packages matching musl headers and libraries together. Target headers,
Blink, and linker assets load lazily. No kernel or operating system boots.

## Use the compiler

Select an example or enter a program with `int main(void)`. Press Ctrl/Cmd+Enter
or click **Compile and run**. **Stop** cancels linking or execution. Linking has
a 30-second limit; execution has a five-second limit. Output is limited to 1 MiB
per stream, and each run starts with a fresh virtual filesystem.

Inspect CLIF, Intel/AT&T assembly, machine-code hex, or ELF object hex. Assembly
shows code before relocation, so symbol addresses can be placeholders. Download
object saves the relocatable object, which you can link elsewhere.

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
