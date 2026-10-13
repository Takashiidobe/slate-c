# Blink browser experiment

Run statically linked x86_64 Linux executables in a browser without booting a
kernel. This experiment uses Blink's interpreter and Emscripten filesystem.

## Build and run

Install Emscripten, GNU Make, Git, Python, and `musl-gcc`, then:

```sh
python3 www/blink/build.py
python3 www/serve.py --directory build/blink-demo --port 8001
```

Open <http://127.0.0.1:8001>. Run the bundled example or upload a statically
linked x86_64 Linux executable. Each run uses a fresh worker and filesystem. The
example uses musl for formatted output, allocation, and file I/O.

The build pins Blink's source revision and disables native JIT, threads, process
creation, sockets, and system-mode execution. Other syscall support still
depends on Emscripten. Interactive input is not configured. The main compiler
build uses the same runtime with packaged musl and GNU ld; this standalone page
is for running existing executables.

Blink executes x86_64 guest programs. Native JIT support on an AArch64 host does
not provide AArch64 guest execution.

Blink source and licensing: <https://github.com/jart/blink>. The demo ships its
ISC license alongside the generated runtime.
