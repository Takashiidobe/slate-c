use super::FunctionLowerer;
use crate::diagnostics::{CompilerError, Result};
use cranelift_codegen::ir::{self as clif, InstBuilder, types};
use cranelift_module::Module;
use slate_parser::ir::{self as c, Type};

impl<M: Module> FunctionLowerer<'_, '_, '_, '_, M> {
    pub(super) fn cast(
        &mut self,
        value: clif::Value,
        from: &Type,
        to: clif::Type,
    ) -> Result<clif::Value> {
        let old = self.builder.func.dfg.value_type(value);
        if old == to {
            return Ok(value);
        }
        Ok(if old.is_int() && to.is_int() {
            if old.bits() > to.bits() {
                self.builder.ins().ireduce(to, value)
            } else if self.module.signed(from) {
                self.builder.ins().sextend(to, value)
            } else {
                self.builder.ins().uextend(to, value)
            }
        } else if old.is_int() && to.is_float() {
            if self.module.signed(from) {
                self.builder.ins().fcvt_from_sint(to, value)
            } else {
                self.builder.ins().fcvt_from_uint(to, value)
            }
        } else if old.is_float() && to.is_int() {
            if self.module.signed(from) {
                self.builder.ins().fcvt_to_sint(to, value)
            } else {
                self.builder.ins().fcvt_to_uint(to, value)
            }
        } else if old.bits() < to.bits() {
            self.builder.ins().fpromote(to, value)
        } else {
            self.builder.ins().fdemote(to, value)
        })
    }

    pub(super) fn condition(&mut self, value: &c::Value) -> Result<clif::Value> {
        let value = self.scalar_value(value)?;
        let ty = self.builder.func.dfg.value_type(value);
        Ok(if ty.is_float() {
            let zero = if ty == types::F32 {
                self.builder.ins().f32const(0.0)
            } else {
                self.builder.ins().f64const(0.0)
            };
            self.builder
                .ins()
                .fcmp(clif::condcodes::FloatCC::NotEqual, value, zero)
        } else {
            self.builder
                .ins()
                .icmp_imm_s(clif::condcodes::IntCC::NotEqual, value, 0)
        })
    }
    pub(super) fn arithmetic(
        &mut self,
        op: &c::ArithOp,
        left: &c::Value,
        right: &c::Value,
    ) -> Result<clif::Value> {
        let lhs = self.scalar_value(left)?;
        let rhs = self.scalar_value(right)?;
        let floating = self.builder.func.dfg.value_type(lhs).is_float();
        Ok(match (op, floating) {
            (c::ArithOp::Add, false) => self.builder.ins().iadd(lhs, rhs),
            (c::ArithOp::Sub, false) => self.builder.ins().isub(lhs, rhs),
            (c::ArithOp::Mul, false) => self.builder.ins().imul(lhs, rhs),
            (c::ArithOp::Div, false) if self.module.signed(&left.ty) => {
                self.builder.ins().sdiv(lhs, rhs)
            }
            (c::ArithOp::Div, false) => self.builder.ins().udiv(lhs, rhs),
            (c::ArithOp::Rem, false) if self.module.signed(&left.ty) => {
                self.builder.ins().srem(lhs, rhs)
            }
            (c::ArithOp::Rem, false) => self.builder.ins().urem(lhs, rhs),
            (c::ArithOp::And, false) => self.builder.ins().band(lhs, rhs),
            (c::ArithOp::Or, false) => self.builder.ins().bor(lhs, rhs),
            (c::ArithOp::Xor, false) => self.builder.ins().bxor(lhs, rhs),
            (c::ArithOp::Shl, false) => self.builder.ins().ishl(lhs, rhs),
            (c::ArithOp::Shr, false) if self.module.signed(&left.ty) => {
                self.builder.ins().sshr(lhs, rhs)
            }
            (c::ArithOp::Shr, false) => self.builder.ins().ushr(lhs, rhs),
            (c::ArithOp::Add, true) => self.builder.ins().fadd(lhs, rhs),
            (c::ArithOp::Sub, true) => self.builder.ins().fsub(lhs, rhs),
            (c::ArithOp::Mul, true) => self.builder.ins().fmul(lhs, rhs),
            (c::ArithOp::Div, true) => self.builder.ins().fdiv(lhs, rhs),
            _ => {
                return Err(CompilerError::Unsupported(format!(
                    "unsupported arithmetic operation: {op}"
                )));
            }
        })
    }
    pub(super) fn unary(
        &mut self,
        op: &c::UnaryArithOp,
        operand: &c::Value,
    ) -> Result<clif::Value> {
        let input = self.scalar_value(operand)?;
        Ok(
            match (op, self.builder.func.dfg.value_type(input).is_float()) {
                (c::UnaryArithOp::Neg, true) => self.builder.ins().fneg(input),
                (c::UnaryArithOp::Neg, false) => self.builder.ins().ineg(input),
                (c::UnaryArithOp::Not, false) if operand.ty == Type::Bool => self
                    .builder
                    .ins()
                    .icmp_imm_s(clif::condcodes::IntCC::Equal, input, 0),
                (c::UnaryArithOp::Not, false) => self.builder.ins().bnot(input),
                _ => {
                    return Err(CompilerError::Unsupported(
                        "unsupported unary operation".to_string(),
                    ));
                }
            },
        )
    }
    pub(super) fn compare(
        &mut self,
        op: &c::CompareOp,
        left: &c::Value,
        right: &c::Value,
        result_type: &Type,
    ) -> Result<clif::Value> {
        let lhs = self.scalar_value(left)?;
        let rhs = self.scalar_value(right)?;
        let boolean = if self.builder.func.dfg.value_type(lhs).is_float() {
            use clif::condcodes::FloatCC;
            let code = match op {
                c::CompareOp::Eq => FloatCC::Equal,
                c::CompareOp::Ne => FloatCC::NotEqual,
                c::CompareOp::Lt => FloatCC::LessThan,
                c::CompareOp::Le => FloatCC::LessThanOrEqual,
                c::CompareOp::Gt => FloatCC::GreaterThan,
                c::CompareOp::Ge => FloatCC::GreaterThanOrEqual,
            };
            self.builder.ins().fcmp(code, lhs, rhs)
        } else {
            use clif::condcodes::IntCC;
            let code = match (op, self.module.signed(&left.ty)) {
                (c::CompareOp::Eq, _) => IntCC::Equal,
                (c::CompareOp::Ne, _) => IntCC::NotEqual,
                (c::CompareOp::Lt, true) => IntCC::SignedLessThan,
                (c::CompareOp::Le, true) => IntCC::SignedLessThanOrEqual,
                (c::CompareOp::Gt, true) => IntCC::SignedGreaterThan,
                (c::CompareOp::Ge, true) => IntCC::SignedGreaterThanOrEqual,
                (c::CompareOp::Lt, false) => IntCC::UnsignedLessThan,
                (c::CompareOp::Le, false) => IntCC::UnsignedLessThanOrEqual,
                (c::CompareOp::Gt, false) => IntCC::UnsignedGreaterThan,
                (c::CompareOp::Ge, false) => IntCC::UnsignedGreaterThanOrEqual,
            };
            self.builder.ins().icmp(code, lhs, rhs)
        };
        let target = self.module.scalar(result_type)?;
        Ok(if target == types::I8 {
            boolean
        } else {
            self.builder.ins().uextend(target, boolean)
        })
    }
}
