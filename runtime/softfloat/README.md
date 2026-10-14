# F80 runtime helpers

Berkeley SoftFloat supplies extended-80 arithmetic for the x86_64 runtime.
Cargo builds and links the helpers for native builds. Build a standalone
archive with:

```sh
python3 runtime/softfloat/build_runtime.py --output build/softfloat
```

This requires Clang and `llvm-ar`. Cargo native builds use the `cc` crate and
its compiler environment variables. Native builds target x86_64 Linux.
Browser compiler builds skip C compilation; `www/runtime/build.py` builds
the helpers for the x86_64 musl guest and includes them in `libslate.a`.

The compiler does not yet lower C `long double` expressions to these helpers.
Use [helpers.h](helpers.h) for the runtime ABI. Native JIT execution registers
the helper symbols, and native filetests link the helper archive.

## Value representation

F80 arguments and results use pointers to 16-byte buffers. The significand
occupies bytes 0–7 and the sign/exponent bytes 8–9, in little-endian order.
Input padding is ignored; outputs clear bytes 10–15. Buffers may be unaligned,
and outputs may alias inputs. No helper accepts or returns native `long double`.

Helpers cover arithmetic, square root, negation, comparisons, and conversions
to/from 64-bit integers, `float`, and `double`. Integer conversions truncate
toward zero. Pseudo-denormals are normalized; unsupported encodings raise the
invalid flag and become a quiet NaN when loaded for arithmetic or conversion.
Negation changes the sign bit without interpreting the encoding.

Rounding and exception flags are thread-local software state. Rounding modes
are 0 (nearest/even), 1 (toward zero), 2 (down), 3 (up), and 4 (nearest/away).
Invalid mode requests return -1 without changing the mode. F80 arithmetic uses
full extended precision. Flag bits are 1 (inexact), 2 (underflow), 4 (overflow),
8 (division by zero), and 16 (invalid). The flags accumulate until cleared.
Equality signals on signaling NaNs; ordered comparisons also signal on quiet
NaNs. Integration with C `fenv`, native x87 call/return adapters, decimal I/O,
and long-double math libraries is separate work.

## Update the dependency

[SOURCE.json](SOURCE.json) pins the official release archive and its SHA-256.
`vendor/` contains the sources listed in `sources.txt` and their headers, using
the upstream x86_64 `SOFTFLOAT_FAST_INT64` configuration and `8086-SSE`
specialization. Algorithms are unchanged; line endings and final blank lines are normalized.

When updating, verify the archive hash, preserve the upstream license, and
check the helper dependency list against the upstream build. `defines.txt`
and `platform.h` are shared by the Cargo and standalone builds. Run the native
F80 filetests and inspect the guest archive before committing an update.

Redistributions must include [SoftFloat's license](vendor/COPYING.txt).
Builders copy it beside the standalone/native archive, and the browser runtime
includes it under `usr/share/licenses/slate-softfloat/`.
