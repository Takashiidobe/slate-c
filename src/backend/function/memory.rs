use super::FunctionLowerer;
use anyhow::{Context, Result, bail, ensure};
use cranelift_codegen::ir::{
    self as clif, InstBuilder, MemFlagsData, StackSlotData, StackSlotKind, types,
};
use cranelift_module::{DataDescription, Module};
use slate_parser::ir::{self as c, BindingId, PlaceKind, Type};

impl<M: Module> FunctionLowerer<'_, '_, '_, '_, M> {
    pub(super) fn local(&mut self, id: BindingId, ty: &Type) -> Result<clif::Value> {
        let slot = if let Some(slot) = self.locals.get(&id) {
            *slot
        } else {
            let size = u32::try_from(self.module.size_of(ty)?)?;
            let slot = self.builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                size.max(1),
                3,
            ));
            self.locals.insert(id, slot);
            slot
        };
        Ok(self.builder.ins().stack_addr(self.pointer, slot, 0))
    }

    pub(super) fn place(&mut self, place: &c::Place) -> Result<clif::Value> {
        ensure!(
            !place.access.atomic && !place.access.volatile,
            "atomic and volatile access is not supported"
        );
        match &place.kind {
            PlaceKind::Binding(id) => {
                if let Some(data) = self.module.symbols.globals.get(id) {
                    let reference = self
                        .module
                        .backend
                        .declare_data_in_func(*data, self.builder.func);
                    Ok(self.builder.ins().symbol_value(self.pointer, reference))
                } else {
                    self.local(*id, &place.ty)
                }
            }
            PlaceKind::Deref(value) => self.value(value),
            PlaceKind::Index { base, index } => {
                let base = self.value(base)?;
                let index_value = self.value(index)?;
                let index_value = self.cast(index_value, &index.ty, self.pointer)?;
                let offset = self
                    .builder
                    .ins()
                    .imul_imm_s(index_value, self.module.size_of(&place.ty)? as i64);
                Ok(self.builder.ins().iadd(base, offset))
            }
            PlaceKind::CompoundLiteral {
                object,
                initializer,
                storage,
                ..
            } => {
                if *storage == c::StorageDuration::Static {
                    let size = self.module.size_of(&place.ty)?;
                    let data = self.module.backend.declare_anonymous_data(false, false)?;
                    let mut description = DataDescription::new();
                    description.set_align(8);
                    description.define(
                        self.module
                            .initial_bytes(initializer, size)?
                            .into_boxed_slice(),
                    );
                    self.module.backend.define_data(data, &description)?;
                    let reference = self
                        .module
                        .backend
                        .declare_data_in_func(data, self.builder.func);
                    Ok(self.builder.ins().symbol_value(self.pointer, reference))
                } else {
                    let pointer = self.local(*object, &place.ty)?;
                    self.initialize(pointer, &place.ty, initializer)?;
                    Ok(pointer)
                }
            }
            PlaceKind::Temporary {
                object,
                initializer,
            } => {
                let pointer = self.local(*object, &place.ty)?;
                self.initialize(pointer, &place.ty, initializer)?;
                Ok(pointer)
            }
            PlaceKind::Field {
                base,
                index,
                bits: None,
            } => {
                let Type::Defined(id) = &base.ty else {
                    bail!("expected record type");
                };
                let definition = self
                    .module
                    .module
                    .types
                    .iter()
                    .find(|ty| ty.value.id == *id)
                    .context("missing record")?;
                let c::TypeDefinitionKind::Record {
                    layout: Some(layout),
                    ..
                } = &definition.value.kind
                else {
                    bail!("incomplete record");
                };
                let offset = layout.offsets[*index];
                let pointer = self.place(base)?;
                Ok(self.builder.ins().iadd_imm_s(pointer, offset as i64))
            }
            other => bail!("unsupported place: {other:?}"),
        }
    }

    pub(super) fn initialize(
        &mut self,
        pointer: clif::Value,
        ty: &Type,
        initializer: &c::Value,
    ) -> Result<()> {
        if matches!(self.module.resolved(ty)?, Type::Array { .. }) {
            let bytes = self
                .module
                .initial_bytes(initializer, self.module.size_of(ty)?)?;
            for (offset, byte) in bytes.into_iter().enumerate() {
                let value = self.builder.ins().iconst(types::I8, i64::from(byte));
                self.builder
                    .ins()
                    .store(MemFlagsData::new(), value, pointer, offset as i32);
            }
        } else {
            let value = self.value(initializer)?;
            self.builder
                .ins()
                .store(MemFlagsData::new(), value, pointer, 0);
        }
        Ok(())
    }
}
