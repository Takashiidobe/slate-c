use super::ModuleLowerer;
use crate::diagnostics::{CompilerError, Result};
use cranelift_module::{DataDescription, Module};
use slate_parser::ir::{self as c, NumericType, Type, ValueKind};

impl<M: Module> ModuleLowerer<'_, '_, M> {
    pub(super) fn initial_bytes(&self, value: &c::Value, size: u64) -> Result<Vec<u8>> {
        self.initial_bytes_inner(value, size)
            .map_err(|error| error.at(value.node.expansion))
    }

    fn initial_bytes_inner(&self, value: &c::Value, size: u64) -> Result<Vec<u8>> {
        match &value.node.value {
            ValueKind::Constant(c::Number::Integer(number)) => {
                let mut bytes = number.to_bytes_le();
                bytes.resize(size as usize, 0);
                Ok(bytes)
            }
            ValueKind::Constant(c::Number::SignedInteger(number)) => {
                let mut bytes = number.to_signed_bytes_le();
                let fill = if number.to_string().starts_with('-') {
                    255
                } else {
                    0
                };
                bytes.resize(size as usize, fill);
                Ok(bytes)
            }
            ValueKind::Constant(c::Number::FloatBits(bits)) => {
                Ok(bits.to_le_bytes()[..size as usize].to_vec())
            }
            ValueKind::CodeUnits(units) => {
                let Type::Array { element, .. } = &value.ty else {
                    return Err(CompilerError::InvalidIr(
                        "expected string array".to_string(),
                    ));
                };
                let width = match element.as_ref() {
                    Type::Numeric(NumericType::Integer { width, .. }) => width / 8,
                    _ => {
                        return Err(CompilerError::Unsupported(
                            "unsupported string encoding".to_string(),
                        ));
                    }
                };
                let mut bytes = Vec::new();
                for unit in units {
                    bytes.extend_from_slice(&unit.to_le_bytes()[..width as usize]);
                }
                bytes.resize(size as usize, 0);
                Ok(bytes)
            }
            ValueKind::Copy { operand, .. } | ValueKind::Convert { operand, .. } => {
                self.initial_bytes(operand, size)
            }
            _ => Err(CompilerError::Unsupported(format!(
                "global initializer is not supported: {value}"
            ))),
        }
    }

    pub(super) fn define_globals(&mut self) -> Result<()> {
        for global in &self.module.globals {
            let location = global.expansion;
            (|| -> Result<()> {
                let global = &global.value;
                let id = self.symbols.globals[&global.variable.id];
                if global.definition {
                    if global.symbol.alias.is_some() || global.symbol.ifunc.is_some() {
                        return Err(CompilerError::Unsupported(
                            "global aliases are not supported".to_string(),
                        ));
                    }
                    let layout =
                        self.object_layout(&global.variable.ty, global.variable.alignment)?;
                    let size = layout.size;
                    let mut data = DataDescription::new();
                    data.set_align(layout.alignment);
                    if let Some(value) = &global.variable.initializer {
                        data.define(self.initial_bytes(value, size)?.into_boxed_slice());
                    } else {
                        data.define_zeroinit(size as usize);
                    }
                    self.backend.define_data(id, &data)?;
                }

                Ok(())
            })()
            .map_err(|error| error.at(location))?;
        }
        Ok(())
    }
}
