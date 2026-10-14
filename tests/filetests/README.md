# Native C filetests

Run `cargo test --test filetests`. The harness discovers every `.c` file in
this directory recursively. Filter by path with
`cargo test --test filetests -- runtime/control`.

Execution requires x86_64 Linux and a C linker (`CC`, default `cc`). Each
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
