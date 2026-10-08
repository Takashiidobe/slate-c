# Browser compiler

Build the compiler as a WASI command and serve the site with isolation headers:

```sh
rustup target add wasm32-wasip1
./www/build.sh
python3 www/serve.py
```

Generated assets live in `dist/` and are ignored by Git. `CLIF_EXPLORER`
overrides the sibling explorer checkout path. `SLATE_HEADERS` overrides Slate's
data directory containing `compiler-headers/` and `sysroots/`. Install Slate's
Clang builtin headers and x86-64/AArch64 GNU Linux sysroots before building.

The build copies existing QEMU/Capstone JS/Wasm and packages target header
sources. It does not rebuild emulators or ship Linux kernels, libc, or a linker.
Header archives load lazily and are mounted through Slate's browser WASI shim.
The C layout profiles are GNU/Linux SysV and AAPCS64 with `-ffreestanding`;
execution is bare-metal and supplies no Linux services.

The browser ELF loader lays out code, data, and BSS and applies relocations.
An entry adapter calls `main` and returns its result through the explorer's
startup stub. Referenced undefined symbols are rejected before downloading
or starting QEMU. Capstone disassembles pre-relocation function bytes, so symbol
addresses in that view are placeholders. Select Intel or AT&T syntax for
x86-64 without recompiling. Machine code shows each function’s bytes; ELF object
shows the complete downloadable object as hexadecimal.

## Hosting

Serve every response with:

```text
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
```

`serve.py` and `dist/_headers` provide these headers. Plain
`python3 -m http.server` does not. The emulator requires WebAssembly Memory64
and shared memory; use a current Chromium or Firefox. The first run downloads
the selected emulator, which is tens of megabytes. Enable HTTP compression.

The UI preserves source and all settings in local storage. Flags are
comma-separated; quote values containing commas. **Share** updates the current
URL and copies a link containing the source and settings. Shared URL settings
take precedence over local storage. Source or flag
changes invalidate previous results and downloads. **Stop** cancels loading or
execution. A five-second execution timeout terminates the guest worker. Every
run starts with fresh guest memory.

QEMU and Capstone licenses and source references are in `dist/licenses`.
The copied startup stubs and worker sources come from `../clif-explorer/web`;
its stub build instructions are in `../clif-explorer/web/stubs`.
