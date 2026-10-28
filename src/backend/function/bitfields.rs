use super::FunctionLowerer;
use crate::diagnostics::{CompilerError, Result};
use cranelift_codegen::ir::{self as clif, InstBuilder, MemFlagsData, types};
use cranelift_module::Module;
use slate_parser::ir::{self as c, Type};

impl<M: Module> FunctionLowerer<'_, '_, '_, '_, M> {
    fn bitfield_storage_type(&self, bits: &c::BitFieldAccess) -> Result<(clif::Type, u32)> {
        Ok(match bits.unit_size {
            1 => (types::I8, 8),
            2 => (types::I16, 16),
            4 => (types::I32, 32),
            8 => (types::I64, 64),
            size => {
                return Err(CompilerError::Unsupported(format!(
                    "bit-field storage unit of {size} bytes is not supported"
                )));
            }
        })
    }

    fn validate_bitfield(&self, bits: &c::BitFieldAccess) -> Result<()> {
        if bits.width > 64 {
            return Err(CompilerError::Unsupported(format!(
                "bit-field width {} is not supported",
                bits.width
            )));
        }
        let end = bits.bit_offset.checked_add(u64::from(bits.width));
        let storage_bits = bits.unit_size.checked_mul(8);
        if bits.width == 0 || end.zip(storage_bits).is_none_or(|(end, size)| end > size) {
            return Err(CompilerError::InvalidIr(
                "bit-field does not fit its storage unit".into(),
            ));
        }
        Ok(())
    }

    pub(super) fn read_bitfield(
        &mut self,
        address: clif::Value,
        bits: &c::BitFieldAccess,
        field_ty: &Type,
        output_ty: clif::Type,
    ) -> Result<clif::Value> {
        self.validate_bitfield(bits)?;
        if !matches!(bits.unit_size, 1 | 2 | 4 | 8) {
            return self.read_packed_bitfield(address, bits, field_ty, output_ty);
        }
        let (storage_ty, storage_bits) = self.bitfield_storage_type(bits)?;
        let mut result = self
            .builder
            .ins()
            .load(storage_ty, MemFlagsData::new(), address, 0);
        if bits.bit_offset != 0 {
            result = self
                .builder
                .ins()
                .ushr_imm_s(result, bits.bit_offset as i64);
        }
        if bits.width < storage_bits {
            let mask = (1u64 << bits.width) - 1;
            result = self.builder.ins().band_imm_s(result, mask as i64);
            if self.module.signed(field_ty) {
                let shift = storage_bits - bits.width;
                result = self.builder.ins().ishl_imm_s(result, shift as i64);
                result = self.builder.ins().sshr_imm_s(result, shift as i64);
            }
        }
        self.cast(result, field_ty, output_ty)
    }

    pub(super) fn write_bitfield(
        &mut self,
        address: clif::Value,
        bits: &c::BitFieldAccess,
        source: clif::Value,
        source_ty: &Type,
    ) -> Result<()> {
        self.validate_bitfield(bits)?;
        if !matches!(bits.unit_size, 1 | 2 | 4 | 8) {
            return self.write_packed_bitfield(address, bits, source, source_ty);
        }
        let (storage_ty, storage_bits) = self.bitfield_storage_type(bits)?;
        let width_mask = if bits.width == 64 {
            u64::MAX
        } else {
            (1u64 << bits.width) - 1
        };
        let field_mask = width_mask << bits.bit_offset;
        let mut value = self.cast(source, source_ty, storage_ty)?;
        if bits.width < storage_bits {
            value = self.builder.ins().band_imm_s(value, width_mask as i64);
        }
        if bits.bit_offset != 0 {
            value = self.builder.ins().ishl_imm_s(value, bits.bit_offset as i64);
        }
        let current = self
            .builder
            .ins()
            .load(storage_ty, MemFlagsData::new(), address, 0);
        let preserved = self.builder.ins().band_imm_s(current, !field_mask as i64);
        let merged = self.builder.ins().bor(preserved, value);
        self.builder
            .ins()
            .store(MemFlagsData::new(), merged, address, 0);
        Ok(())
    }

    pub(super) fn normalize_bitfield_value(
        &mut self,
        value: clif::Value,
        bits: &c::BitFieldAccess,
        field_ty: &Type,
    ) -> clif::Value {
        let value_bits = self.builder.func.dfg.value_type(value).bits();
        let mask = if bits.width == 64 {
            u64::MAX
        } else {
            (1u64 << bits.width) - 1
        };
        let mut result = if bits.width < value_bits {
            self.builder.ins().band_imm_s(value, mask as i64)
        } else {
            value
        };
        if self.module.signed(field_ty) && bits.width < value_bits {
            let shift = value_bits - bits.width;
            result = self.builder.ins().ishl_imm_s(result, shift as i64);
            result = self.builder.ins().sshr_imm_s(result, shift as i64);
        }
        result
    }

    fn read_packed_bitfield(
        &mut self,
        address: clif::Value,
        bits: &c::BitFieldAccess,
        field_ty: &Type,
        output_ty: clif::Type,
    ) -> Result<clif::Value> {
        let ty = if bits.width <= 32 {
            types::I32
        } else {
            types::I64
        };
        let mut result = self.builder.ins().iconst(ty, 0);
        let end = bits.bit_offset + u64::from(bits.width);
        for byte in bits.bit_offset / 8..end.div_ceil(8) {
            let first = bits.bit_offset.max(byte * 8);
            let width = end.min(byte * 8 + 8) - first;
            let offset = i32::try_from(byte)?;
            let loaded = self
                .builder
                .ins()
                .load(types::I8, MemFlagsData::new(), address, offset);
            let mut value = self.builder.ins().uextend(ty, loaded);
            if first % 8 != 0 {
                value = self.builder.ins().ushr_imm_s(value, (first % 8) as i64);
            }
            value = self
                .builder
                .ins()
                .band_imm_s(value, ((1u64 << width) - 1) as i64);
            if first > bits.bit_offset {
                value = self
                    .builder
                    .ins()
                    .ishl_imm_s(value, (first - bits.bit_offset) as i64);
            }
            result = self.builder.ins().bor(result, value);
        }
        let result = self.normalize_bitfield_value(result, bits, field_ty);
        self.cast(result, field_ty, output_ty)
    }

    fn write_packed_bitfield(
        &mut self,
        address: clif::Value,
        bits: &c::BitFieldAccess,
        source: clif::Value,
        source_ty: &Type,
    ) -> Result<()> {
        let ty = if bits.width <= 32 {
            types::I32
        } else {
            types::I64
        };
        let source = self.cast(source, source_ty, ty)?;
        let end = bits.bit_offset + u64::from(bits.width);
        for byte in bits.bit_offset / 8..end.div_ceil(8) {
            let first = bits.bit_offset.max(byte * 8);
            let width = end.min(byte * 8 + 8) - first;
            let mask = ((1u64 << width) - 1) << (first % 8);
            let mut value = source;
            if first > bits.bit_offset {
                value = self
                    .builder
                    .ins()
                    .ushr_imm_s(value, (first - bits.bit_offset) as i64);
            }
            let mut value = self.builder.ins().ireduce(types::I8, value);
            if first % 8 != 0 {
                value = self.builder.ins().ishl_imm_s(value, (first % 8) as i64);
            }
            let offset = i32::try_from(byte)?;
            if width < 8 {
                value = self.builder.ins().band_imm_s(value, mask as i64);
                let current =
                    self.builder
                        .ins()
                        .load(types::I8, MemFlagsData::new(), address, offset);
                let preserved = self.builder.ins().band_imm_s(current, !(mask as i64));
                value = self.builder.ins().bor(preserved, value);
            }
            self.builder
                .ins()
                .store(MemFlagsData::new(), value, address, offset);
        }
        Ok(())
    }
}
