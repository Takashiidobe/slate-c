# Browser bare-metal QEMU handoff

Compile C locally to x86-64 or AArch64 machine code and run `int main(void)`
directly under QEMU. The user replaced the Linux/musl plan with bare-metal
execution because Linux startup took minutes. Keep this document current and
commit each completed step. Do not add smoke tests; the user will test the UI.

## Build and use

```sh
./www/build.sh
python3 www/serve.py
```

Open <http://127.0.0.1:8000>. Choose a target and example, then use **Compile and
run** or Ctrl/Cmd+Enter. Source/flag edits refresh compiler output. The selected
emulator loads lazily when running. **Stop** cancels execution; guest execution
has a five-second limit.

## Agreed scope

- Support x86-64 and AArch64 together.
- Separate C compiler flags from Cranelift settings in the UI.
- Pass frontend argv directly to Slate and backend settings to Cranelift.
- Preserve native command defaults and their original lowering for now.
- Reuse the explorer's built QEMU/Capstone assets without rebuilding them.
- Reuse Slate's WASI shim and lazy header archives.
- Remove clif2wasm/Waffle; generated programs are native ELF objects.
- Execute without an OS, libc, an ELF linker binary, or Linux runtime assets.

## Architecture

```text
C + frontend flags -> Slate IR -> CLIF + backend flags -> ELF object
ELF sections + relocations -> browser loader -> guest code and data image
startup stub -> generated entry adapter -> main -> serial result record
machine code -> Capstone -> assembly view
```

The compiler is a WASI command. The browser mounts target header sources and
Clang builtin headers through the existing filesystem shim. Target profiles
use GNU/Linux SysV and AAPCS64 C layouts with `-ffreestanding`; QEMU execution
has no Linux services. Header declarations do not provide implementations.

The compiler emits a small entry adapter which calls parameterless `main`,
stores its signed i32 result at the stub's result offset, and returns the result
buffer pointer. The JavaScript ELF loader lays out allocated code/data/BSS
sections, resolves symbols, and applies native relocations, including GOT
references. Referenced undefined symbols produce an explicit error before the
emulator downloads or starts.

The explorer's original startup stubs and QEMU worker provide guest setup,
serial result/trap records, and timeout termination. Each run gets fresh guest
memory. Function and data images are limited to 4 MiB to preserve the stub's
reserved memory and stack layout.

## Milestones

- [x] WASI compiler, two native backends, frontend/backend flag arrays.
- [x] Scalar functions, calls, globals, locals, strings, pointers, arithmetic,
  conditionals, and structured loops.
- [x] Cranelift setting schemas, machine code, and Capstone disassembly.
- [x] Remove clif2wasm/Waffle and obsolete Wasm-emission APIs/smoke scripts.
- [x] Generate an entry adapter and add native ELF section/relocation loading.
- [x] Replace Linux execution with the explorer's bare-metal stubs/workers.
- [x] Copy original QEMU/Capstone artifacts and package lazy target headers.
- [x] Replace libc examples and runtime output/download controls with
  freestanding examples, main's return value, and ELF object downloads.
- [x] The built bare-metal worker returns 42 on both copied Wasm emulators
  for a program combining helpers, globals, locals, loops, and string data.
- [x] Commit the runner/build/UI steps and record final validation.

## Commit history

- `745a856`: WASI compiler foundation and initial handoff.
- `22b42d8`: enable musl profiles during the superseded Linux experiment.
- Sibling Slate `e6cb3aad5`: Clang musl profiles, retained as independent work.
- `51b70ea`: scalar object lowering, machine code, and flag schema.
- `44f9c49`: remove the Wasm bridge and emission command.
- `3d070de`: fix boolean negation and apply Cranelift presets.
- `3339efc`: bare-metal entry adapter, ELF loader, startup stubs, and runner.
- Browser build/UI step: copied assets, lazy headers, flags, binary views, and
  Intel/AT&T syntax (commit containing this handoff update).

## Validation

Existing all-feature Rust tests and strict Clippy pass. The built WASI worker
has parsed real header sources, emitted both ELF architectures, generated both
flag schemas, and disassembled through the copied Capstone. Bare-metal runtime
checks of helpers, globals, locals, loops, and strings return 42 on both
architectures using the built WASI compiler and original copied emulators:
x86-64 about 450 ms; AArch64 about 862 ms in the local Node-hosted check.
These include emulator loading/initialization; browser timings may differ.

## Limits

Execution requires `int main(void)`. Unresolved external functions and data
are rejected; `printf`, `malloc`, libc, and syscalls are unavailable. Variadic
calls/definitions, aggregate function ABI passes, extended floats,
atomic/volatile access, inline assembly, switch/goto, TLS, static locals, and
some initializer forms remain unsupported.

Assembly shows pre-relocation function bytes, so external addresses are
placeholders. Backend optimization uses Cranelift settings; unsupported C
optimization/codegen options are rejected. Native commands retain the original
constant-return subset.

The browser needs Memory64 and cross-origin isolation. Use `www/serve.py` or
host with `dist/_headers`; enable HTTP compression for the large emulator
assets. QEMU and Capstone licenses and emulator source references ship in
`dist/licenses`. No new test scripts are required for browser handoff.

## Deferred Linux work

The uncommitted Linux/musl launcher, asset builder, poll fixes, and worker were
moved to the ignored local `build/deferred-linux` directory. They are not
packaged by the site or required by the build. The earlier musl profiles remain
committed in Slate, but the active UI uses GNU C profiles. A future small
bare-metal I/O/heap runtime can be added independently of libc or Linux.

## Binary inspection

The output selector includes CLIF, assembly, per-function machine-code hex, and
complete ELF object hex. x86-64 assembly supports Intel and AT&T syntax; both
are generated with Capstone and switching syntax requires no compilation.

Final browser build and JavaScript syntax checks pass. A local check of the
rebuilt worker verifies distinct Intel/AT&T listings, both hexadecimal views,
and result 42 on both architectures (481 ms x86-64; 829 ms AArch64). The
isolated development server is available at http://127.0.0.1:8000 for the
user’s browser checks. No smoke test scripts were added.

## Output highlighting

Custom CodeMirror languages highlight CLIF and assembly; hexadecimal views
remain plain text. The CLIF tokens follow `../tree-sitter-cranelift/grammar.js`
and its highlight query, with vector types and numeric exponents supported.
Assembly highlights Intel, AT&T, and AArch64 instructions, registers, immediates,
and labels. Assembly shows instructions without byte offsets or hex bytes;
those remain in the machine-code and ELF hex views. The sibling grammar’s MIT license ships in assets.
JavaScript syntax, representative tokenization checks, and the browser build
pass. Browser visual checks remain with the user.

## UI simplification

The header is Slate C Compiler with no subheading. Targets display x86_64 and
aarch64. The capability footer and ELF size indicator are removed. Execution
uses Ready/Waiting for idle and pending states and retains main’s return value
after a run. Machine-code and ELF hexadecimal views show eight bytes per row.
The browser build and JavaScript syntax checks pass.

## Sharing and persistence

Share updates the current URL and copies it with source, target, comma-separated
c-flags/clif-flags, output view, syntax, key bindings, example selection, filter,
and settings-panel visibility. Shared values override local storage; all UI
settings persist locally. Previous space-separated saved flags are migrated.
Quoted commas are supported in flag values. Flag parsing, legacy migration,
settings URL round-trip checks, JavaScript syntax, and the browser build pass.
