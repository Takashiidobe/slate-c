use super::{FunctionLowerer, LoweredValue};
use crate::diagnostics::{CompilerError, Result};
use cranelift_codegen::ir::{
    self as clif, InstBuilder, MemFlagsData, StackSlotData, StackSlotKind, types,
};
use cranelift_module::{DataDescription, Module};
use slate_parser::ir::{self as c, BindingId, PlaceKind, Type};

impl<M: Module> FunctionLowerer<'_, '_, '_, '_, M> {
    fn stack_slot(&mut self, ty: &Type) -> Result<clif::StackSlot> {
        let size = u32::try_from(self.module.size_of(ty)?)?;
        Ok(self.builder.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            size.max(1),
            3,
        )))
    }

    pub(super) fn temporary(&mut self, ty: &Type) -> Result<clif::Value> {
        let slot = self.stack_slot(ty)?;
        Ok(self.builder.ins().stack_addr(self.pointer, slot, 0))
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

    pub(super) fn local(&mut self, id: BindingId, ty: &Type) -> Result<clif::Value> {
        let slot = if let Some(slot) = self.locals.get(&id) {
            *slot
        } else {
            let slot = self.stack_slot(ty)?;
            self.locals.insert(id, slot);
            slot
        };
        Ok(self.builder.ins().stack_addr(self.pointer, slot, 0))
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
                    self.local(*id, &place.ty)
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
                    return Err(CompilerError::InvalidIr("expected record type".to_string()));
                };
                let definition = self
                    .module
                    .module
                    .types
                    .iter()
                    .find(|ty| ty.value.id == *id)
                    .ok_or_else(|| CompilerError::InvalidIr("missing record".into()))?;
                let c::TypeDefinitionKind::Record {
                    layout: Some(layout),
                    ..
                } = &definition.value.kind
                else {
                    return Err(CompilerError::InvalidIr("incomplete record".to_string()));
                };
                let offset = layout.offsets[*index];
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
