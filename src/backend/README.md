# Extending the backend

This backend lowers Slate translation units into Cranelift functions and data.
Native JIT execution and browser object compilation use the same lowering code.

Run the backend regression tests after changing a lowering rule:

```sh
cargo test --all-features
```

## Add an operation

Handle the Slate variant in `function/value.rs` or `function/control.rs`. Keep
expression and statement matches exhaustive so dependency updates expose new
variants during compilation. Add the implementation to the relevant module:

- `function/arithmetic.rs`: arithmetic, comparisons, and scalar conversions.
- `function/memory.rs`: local storage, place addresses, and initialization.
- `function/calls.rs`: call emission and argument evaluation.
- `function/control.rs`: blocks, branches, loops, and statement dispatch.

Use `ModuleLowerer` for module state, layout, ABI, data, and symbol helpers.
Implement its methods in the relevant module and read the Slate IR through
`self.module`; use `self.backend` for Cranelift module access.

Use the shared `FunctionLowerer` for builder access and per-function state.
Its `self.module` borrows the owning `ModuleLowerer`, so call helpers such as
`self.module.scalar(ty)` without passing the Slate module. Add
an execution test in `tests/pipeline.rs` and a cross-compilation case in
`tests/cross.rs` when the change affects object output or calls.

## Extend types, calls, or globals

Keep type resolution and object sizes in `layout.rs`. Use Slate's record layout
when accessing fields. Change `abi.rs` for signatures shared by definitions and
calls; change `runtime.rs` for helper symbol conventions.

Use `data.rs` for static initializer encoding and relocations. Use `symbols.rs`
for module declarations and linkage. Keep module orchestration in `mod.rs`.

Values currently lower to scalar Cranelift values, and places lower to
addresses. Extend those representations when adding aggregates, split values,
or access metadata; update their consumers together.

## Keep execution requirements in the driver

Object compilation accepts translation units without `main`. Native execution
checks its callable entry point in `../entry.rs` before lowering and invoking
it. Keep linking, symbol resolution, and serialization in the drivers.

The public standalone `lower` API in `../standalone.rs` retains its original
constant-return subset. Add compiler features to this shared backend.
