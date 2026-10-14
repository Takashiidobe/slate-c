use super::{FunctionLowerer, LocalSlot, LoweredValue};
use crate::diagnostics::{CompilerError, Result};
use cranelift_codegen::ir::{
    self as clif, InstBuilder, MemFlagsData, StackSlotData, StackSlotKind, types,
};
use cranelift_module::{DataDescription, Module};
use slate_parser::ir::{self as c, BindingId, PlaceKind, Type};

impl<M: Module> FunctionLowerer<'_, '_, '_, '_, M> {
    fn stack_slot(&mut self, ty: &Type, alignment: Option<u64>) -> Result<LocalSlot> {
        let layout = self.module.object_layout(ty, alignment)?;
        let padding = if layout.alignment > 16 {
            layout.alignment - 1
        } else {
            0
        };
        let size = layout.size.max(1).checked_add(padding).ok_or_else(|| {
            CompilerError::Unsupported("aligned stack object is too large".into())
        })?;
        let slot = self.builder.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            u32::try_from(size)?,
            layout.alignment.min(16).trailing_zeros() as u8,
        ));
        Ok(LocalSlot {
            slot,
            alignment: layout.alignment,
        })
    }

    fn local_address(&mut self, local: LocalSlot) -> clif::Value {
        let address = self.builder.ins().stack_addr(self.pointer, local.slot, 0);
        if local.alignment <= 16 {
            return address;
        }
        let address = self
            .builder
            .ins()
            .iadd_imm_s(address, (local.alignment - 1) as i64);
        self.builder
            .ins()
            .band_imm_s(address, -(local.alignment as i64))
    }

    pub(super) fn temporary(&mut self, ty: &Type) -> Result<clif::Value> {
        let slot = self.stack_slot(ty, None)?;
        Ok(self.local_address(slot))
    }

    pub(super) fn copy_object(
        &mut self,
        destination: clif::Value,
        source: clif::Value,
        ty: &Type,
    ) -> Result<()> {
        let size = self.module.size_of(ty)?;
        self.builder.emit_small_memory_copy(
            self.module.backend.isa().frontend_config(),
            destination,
            source,
            size,
            1,
            1,
            true,
            MemFlagsData::new(),
        );
        Ok(())
    }

    pub(super) fn write_value(
        &mut self,
        pointer: clif::Value,
        ty: &Type,
        value: LoweredValue,
    ) -> Result<()> {
        match value {
            LoweredValue::Scalar(value) => {
                if self.module.aggregate(ty)? {
                    return Err(CompilerError::InvalidIr(
                        "expected aggregate value for store".to_string(),
                    ));
                };
                self.builder
                    .ins()
                    .store(MemFlagsData::new(), value, pointer, 0);
            }
            LoweredValue::Aggregate { address } => {
                if !self.module.aggregate(ty)? {
                    return Err(CompilerError::InvalidIr(
                        "expected scalar value for store".to_string(),
                    ));
                };
                self.copy_object(pointer, address, ty)?;
            }
            LoweredValue::Void => {
                return Err(CompilerError::InvalidIr(
                    "cannot store void value".to_string(),
                ));
            }
        }
        Ok(())
    }

    pub(super) fn local(
        &mut self,
        id: BindingId,
        ty: &Type,
        alignment: Option<u64>,
    ) -> Result<clif::Value> {
        let slot = if let Some(slot) = self.locals.get(&id) {
            *slot
        } else {
            let slot = self.stack_slot(ty, alignment)?;
            self.locals.insert(id, slot);
            slot
        };
        Ok(self.local_address(slot))
    }

    pub(super) fn place(&mut self, place: &c::Place) -> Result<clif::Value> {
        if place.access.atomic || place.access.volatile {
            return Err(CompilerError::Unsupported(
                "atomic and volatile access is not supported".to_string(),
            ));
        };
        match &place.kind {
            PlaceKind::Binding(id) => {
                if let Some(data) = self.module.symbols.globals.get(id) {
                    let reference = self
                        .module
                        .backend
                        .declare_data_in_func(*data, self.builder.func);
                    Ok(self.builder.ins().symbol_value(self.pointer, reference))
                } else {
                    self.local(*id, &place.ty, None)
                }
            }
            PlaceKind::Deref(value) => self.scalar_value(value),
            PlaceKind::Index { base, index } => {
                let base = self.scalar_value(base)?;
                let index_value = self.scalar_value(index)?;
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
                alignment,
            } => {
                if *storage == c::StorageDuration::Static {
                    let size = self.module.size_of(&place.ty)?;
                    let data = self.module.backend.declare_anonymous_data(false, false)?;
                    let mut description = DataDescription::new();
                    description
                        .set_align(self.module.object_layout(&place.ty, *alignment)?.alignment);
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
                    let pointer = self.local(*object, &place.ty, *alignment)?;
                    self.initialize(pointer, &place.ty, initializer)?;
                    Ok(pointer)
                }
            }
            PlaceKind::Temporary {
                object,
                initializer,
            } => {
                let pointer = self.local(*object, &place.ty, None)?;
                self.initialize(pointer, &place.ty, initializer)?;
                Ok(pointer)
            }
            PlaceKind::Field {
                base,
                index,
                bits: None,
            } => {
                let offset = *self
                    .module
                    .record_layout(&base.ty)?
                    .offsets
                    .get(*index)
                    .ok_or_else(|| {
                        CompilerError::InvalidIr("record field index out of bounds".into())
                    })?;
                let pointer = self.place(base)?;
                Ok(self.builder.ins().iadd_imm_s(pointer, offset as i64))
            }
            other => Err(CompilerError::Unsupported(format!(
                "unsupported place: {other:?}"
            ))),
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
            self.write_value(pointer, ty, value)?;
        }
        Ok(())
    }
}
