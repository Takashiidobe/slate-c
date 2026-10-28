# Native C filetests

Run `cargo test --test filetests`. The harness discovers every `.c` file in
this directory recursively. Filter by path with
`cargo test --test filetests -- runtime/control`.

Execution requires x86_64 or AArch64 Linux and a C linker (`CC`, default `cc`). Each
program runs in a temporary directory with closed stdin and a five-second
timeout. Signals and timeouts fail the test.

```c
// EXIT: 42
// STDOUT: empty
// STDERR: empty
int main(void) { return 42; }
```

`MODE: run` is the default. `EXIT` is required and must be an exact process
exit code in 0..=255. Unspecified stdout and stderr must be empty.
Add `STDOUT: check: text` or `STDERR: check: text` to match output.

Use `MODE: compile` for CLIF fixtures without linking or execution:

```c
// MODE: compile
// CLIF: check: ; add
// CLIF: check: $(sum=$var) = iadd $var, $var
// CLIF: nextln: return $sum
int add(int x, int y) { return x + y; }
```

`$var`, `$block`, and `$func` match generated CLIF values, blocks, and
function references. Capture an identifier with `$(name=$var)` and reuse
it with `$name`. Add channel-specific patterns with
`CLIF: regex: name=pattern`. CLIF records lowering before backend optimization.
Execution fixtures can also include CLIF checks.

Every C fixture under `runtime/builtins/` also runs in a separate JIT process.
Both runs use the same exit, stdout, and stderr checks and five-second timeout.
Add fixtures there to exercise builtin lowering and runtime symbol resolution.

The builtin experiment uses system libc, libm, and GCC's libgcc. Install the
native GCC runtime development files before building; the JIT explicitly links
libm and libgcc, and object tests use the C driver's runtime selection. The
compiler helper fixture requires `__popcountdi2`. These libraries are not bundled.
Cross-target objects require matching target libraries when linked.

The initial builtin support covers scalar elementwise popcount and FMA,
`__builtin_memcpy`, `__builtin_memmove`, `__builtin_memset`,
`__builtin_memcmp`, `__builtin_sqrt[f]`, `__builtin_fma[f]`,
`__builtin_sin`, and `__builtin_cos`. Other remaining `__builtin_` calls and
intrinsic nodes produce lowering diagnostics. Browser builds can emit objects
using the shared lowering; runtime libraries must be supplied when linking.

Use `MODE: error` with required `ERROR: check: text` directives for expected
compiler failures. Checks match rendered miette diagnostics, including source
labels and diagnostic codes. Replace unsupported-feature fixtures with execution
fixtures when implementing the feature.

Each channel accepts Rust FileCheck's `check`, `nextln`, `sameln`,
`unordered`, `not`, and `regex` directives. `empty` requires an empty stream
and cannot be combined with matching directives. See the
[Rust FileCheck syntax](https://docs.rs/filecheck) for regex and capture rules.

Use integration tests and fixtures; do not add unit tests. Native tests are
the routine gate. Run browser tests only for changes to browser integration.
