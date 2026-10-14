# Musl and linker assets

Package the headers, libraries, startup objects, and GNU linker used by the
browser's x86_64 musl target:

```sh
python3 www/runtime/build.py
```

The normal browser build runs this step automatically. Install Clang, `llvm-ar`,
Python, and curl first. Downloads are pinned by URL and SHA-256 in
[sources.json](sources.json), and are reused from `build/runtime/downloads`.

The output is `build/blink-runtime/runtime.tar.gz` plus `headers.tar.gz`. The
runtime contains musl, compiler helpers, GNU ld and its dependencies, and the
SysV variadic-call adapters. It contains no kernel. GNU ld and user programs
execute in separate Blink workers with fresh filesystems.

`libslate.a` also contains the [SoftFloat F80 helpers](../../runtime/softfloat/README.md).
Their pinned source and license are vendored in the repository.

To update packages, update URLs and checksums together in `sources.json` and
check both linking and execution in the browser. Keep musl headers and libraries
from the same package release.
