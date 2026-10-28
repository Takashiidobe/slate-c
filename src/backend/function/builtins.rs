use super::{FunctionLowerer, LoweredValue};
use crate::diagnostics::{CompilerError, Result};
use cranelift_codegen::ir::{InstBuilder, types};
use cranelift_module::Module;
use slate_parser::ir::{self as c, Type};

impl<M: Module> FunctionLowerer<'_, '_, '_, '_, M> {
    pub(super) fn intrinsic(
        &mut self,
        name: &str,
        arguments: &[c::Value],
        result_type: &Type,
    ) -> Result<LoweredValue> {
        let supported = match (name, arguments) {
            ("llvm.ctpop", [operand]) => {
                operand.ty == *result_type
                    && matches!(
                        result_type,
                        Type::Numeric(c::NumericType::Integer {
                            width: 8 | 16 | 32 | 64,
                            ..
                        })
                    )
            }
            ("llvm.fma", [a, b, c]) => {
                a.ty == *result_type && b.ty == *result_type && c.ty == *result_type
            }
            _ => false,
        };
        if !supported {
            return Err(CompilerError::Unsupported(format!(
                "intrinsic {name} with result type {result_type:?} is not supported"
            )));
        }
        let ty = self.module.scalar(result_type)?;
        if (name == "llvm.ctpop" && !matches!(ty, types::I8 | types::I16 | types::I32 | types::I64))
            || (name == "llvm.fma" && !matches!(ty, types::F32 | types::F64))
        {
            return Err(CompilerError::Unsupported(format!(
                "intrinsic {name} with result type {result_type:?} is not supported"
            )));
        }
        let values = arguments
            .iter()
            .map(|argument| self.scalar_value(argument))
            .collect::<Result<Vec<_>>>()?;
        let value = match name {
            "llvm.ctpop" => self.builder.ins().popcnt(values[0]),
            "llvm.fma" => self.builder.ins().fma(values[0], values[1], values[2]),
            _ => unreachable!(),
        };
        Ok(LoweredValue::Scalar(value))
    }
}
