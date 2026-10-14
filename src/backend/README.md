# Extending the backend

This backend lowers Slate translation units into Cranelift functions and data.
Native JIT execution and browser object compilation use the same lowering code.

Run the backend regression tests after changing a lowering rule:

```sh
cargo test
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
an execution fixture under `tests/filetests/` and a focused CLIF fixture when
instruction selection during lowering matters. Use regex captures for generated
identifiers. See the [fixture instructions](../../tests/filetests/README.md).
Do not add unit tests. Browser execution is only needed for browser integration
changes; native fixtures are the primary regression coverage.

## Extend types, calls, or globals

Keep type resolution and object sizes in `layout.rs`. Use Slate's record layout
when accessing fields. Change `abi.rs` for signatures shared by definitions and
calls; change `runtime.rs` for helper symbol conventions.

Use `object_layout` for storage size and alignment, passing Slate's explicit
object alignment when present. Scalar storage comes from Slate's target data;
record size, alignment, and field offsets come from its record layout. Arrays
use the element's padded size as their stride. Stack objects aligned beyond
16 bytes reserve extra space and round their addresses inside the slot.

Use `data.rs` for static initializer encoding and relocations. Use `symbols.rs`
for module declarations and linkage. Keep module orchestration in `mod.rs`.

`LoweredValue` represents void, scalar Cranelift values, and aggregates in
temporary storage. Aggregate reads capture their contents before later side
effects. Places lower to addresses. Use `scalar_value` only for scalar operands
and `write_value` for stores and copies. Extend the representation when adding
split ABI values or access metadata; update its consumers together.

Automatic aggregate initializers write members directly into local storage.
Follow Slate's member order, recurse into nested arrays and records, and clear
the object first when `zero_fill` is set. Copy-valued members use the same
snapshot representation as ordinary aggregate assignments.

Return typed `CompilerError` diagnostics. Attach the innermost Slate expansion
span with `at`; drivers retain source files and attach their contents before
rendering or serialization. Preserve frontend diagnostics and related errors.

## Keep execution requirements in the driver

Object compilation accepts translation units without `main`. Native execution
checks its callable entry point in `../entry.rs` before lowering and invoking
it. Keep linking, symbol resolution, and serialization in the drivers.

The public standalone `lower` API in `../standalone.rs` retains its original
constant-return subset. Add compiler features to this shared backend.
