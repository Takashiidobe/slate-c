use super::FunctionLowerer;
use anyhow::{Context, Result, bail, ensure};
use cranelift_codegen::ir::{self as clif, InstBuilder, MemFlagsData, types};
use cranelift_module::Module;
use slate_parser::ir::{self as c, Type, ValueKind};

impl<M: Module> FunctionLowerer<'_, '_, '_, '_, M> {
    pub(super) fn value(&mut self, value: &c::Value) -> Result<clif::Value> {
        match &value.node.value {
            ValueKind::Constant(number) => {
                let ty = self.module.scalar(&value.ty)?;
                Ok(match number {
                    c::Number::Integer(number) => self
                        .builder
                        .ins()
                        .iconst(ty, number.to_string().parse::<u64>()? as i64),
                    c::Number::SignedInteger(number) => self
                        .builder
                        .ins()
                        .iconst(ty, number.to_string().parse::<i64>()?),
                    c::Number::Bool(value) => self.builder.ins().iconst(ty, i64::from(*value)),
                    c::Number::FloatBits(bits) if ty == types::F32 => self
                        .builder
                        .ins()
                        .f32const(clif::immediates::Ieee32::with_bits(*bits as u32)),
                    c::Number::FloatBits(bits) if ty == types::F64 => self
                        .builder
                        .ins()
                        .f64const(clif::immediates::Ieee64::with_bits(*bits as u64)),
                    _ => bail!("unsupported constant: {number:?}"),
                })
            }
            ValueKind::Copy { operand, .. } => self.value(operand),
            ValueKind::Null | ValueKind::Void => Ok(self.builder.ins().iconst(self.pointer, 0)),
            ValueKind::Read { place, ordering } => {
                ensure!(ordering.is_none(), "atomic loads are not supported");
                let pointer = self.place(place)?;
                Ok(self.builder.ins().load(
                    self.module.scalar(&value.ty)?,
                    MemFlagsData::new(),
                    pointer,
                    0,
                ))
            }
            ValueKind::AddressOf(place) | ValueKind::ArrayDecay { place, .. } => self.place(place),
            ValueKind::Convert { operand, .. } => {
                if value.ty == Type::Bool {
                    let condition = self.condition(operand)?;
                    return Ok(condition);
                }
                let input = self.value(operand)?;
                let to = self.module.scalar(&value.ty)?;
                if self.builder.func.dfg.value_type(input).is_float() && to.is_int() {
                    return Ok(if self.module.signed(&value.ty) {
                        self.builder.ins().fcvt_to_sint(to, input)
                    } else {
                        self.builder.ins().fcvt_to_uint(to, input)
                    });
                }
                self.cast(input, &operand.ty, to)
            }
            ValueKind::Store {
                place,
                value: operand,
                ordering,
            } => {
                ensure!(ordering.is_none(), "atomic stores are not supported");
                let pointer = self.place(place)?;
                let output = self.value(operand)?;
                self.builder
                    .ins()
                    .store(MemFlagsData::new(), output, pointer, 0);
                Ok(output)
            }
            ValueKind::OldValue => self.old.context("old value outside update"),
            ValueKind::Update {
                place,
                computation,
                postfix,
                ordering,
            } => {
                ensure!(ordering.is_none(), "atomic updates are not supported");
                let pointer = self.place(place)?;
                let before = self.builder.ins().load(
                    self.module.scalar(&place.ty)?,
                    MemFlagsData::new(),
                    pointer,
                    0,
                );
                let saved = self.old.replace(before);
                let after = self.value(computation)?;
                self.old = saved;
                self.builder
                    .ins()
                    .store(MemFlagsData::new(), after, pointer, 0);
                Ok(if *postfix { before } else { after })
            }
            ValueKind::Sequence { left, right } => {
                self.value(left)?;
                self.value(right)
            }
            ValueKind::StatementExpression(evaluation) => {
                self.statements(&evaluation.statements)?;
                self.value(&evaluation.value)
            }
            ValueKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                let condition = self.condition(condition)?;
                let yes = self.builder.create_block();
                let no = self.builder.create_block();
                let merge = self.builder.create_block();
                self.builder
                    .append_block_param(merge, self.module.scalar(&value.ty)?);
                self.builder.ins().brif(condition, yes, &[], no, &[]);
                self.builder.switch_to_block(yes);
                let output = self.value(then_value)?;
                self.builder.ins().jump(merge, &[output.into()]);
                self.builder.switch_to_block(no);
                let output = self.value(else_value)?;
                self.builder.ins().jump(merge, &[output.into()]);
                self.builder.switch_to_block(merge);
                Ok(self.builder.block_params(merge)[0])
            }
            ValueKind::Logical { op, left, right } => {
                let left = self.condition(left)?;
                let evaluate = self.builder.create_block();
                let skip = self.builder.create_block();
                let merge = self.builder.create_block();
                self.builder.append_block_param(merge, types::I8);
                if *op == c::LogicalOp::And {
                    self.builder.ins().brif(left, evaluate, &[], skip, &[]);
                } else {
                    self.builder.ins().brif(left, skip, &[], evaluate, &[]);
                }
                self.builder.switch_to_block(skip);
                let constant = self
                    .builder
                    .ins()
                    .iconst(types::I8, i64::from(*op == c::LogicalOp::Or));
                self.builder.ins().jump(merge, &[constant.into()]);
                self.builder.switch_to_block(evaluate);
                let right = self.condition(right)?;
                self.builder.ins().jump(merge, &[right.into()]);
                self.builder.switch_to_block(merge);
                let boolean = self.builder.block_params(merge)[0];
                Ok(self
                    .builder
                    .ins()
                    .uextend(self.module.scalar(&value.ty)?, boolean))
            }
            ValueKind::Arith {
                op, left, right, ..
            } => self.arithmetic(op, left, right),
            ValueKind::Unary { op, operand, .. } => self.unary(op, operand),
            ValueKind::Compare {
                op, left, right, ..
            } => self.compare(op, left, right, &value.ty),
            ValueKind::PointerOffset {
                pointer,
                amount,
                subtract,
                element,
                ..
            } => {
                let base = self.value(pointer)?;
                let offset = self.value(amount)?;
                let offset = self.cast(offset, &amount.ty, self.pointer)?;
                let offset = self
                    .builder
                    .ins()
                    .imul_imm_s(offset, self.module.size_of(element)? as i64);
                Ok(if *subtract {
                    self.builder.ins().isub(base, offset)
                } else {
                    self.builder.ins().iadd(base, offset)
                })
            }
            ValueKind::Call {
                callee: c::Callee::Direct(binding),
                signature,
                abi,
                arguments,
            } => self.call(*binding, signature, abi, arguments, &value.ty),
            unsupported @ (ValueKind::LabelAddress(..)
            | ValueKind::Seh(..)
            | ValueKind::CodeUnits(..)
            | ValueKind::Aggregate { .. }
            | ValueKind::FunctionDecay { .. }
            | ValueKind::CompareExchange { .. }
            | ValueKind::Fence { .. }
            | ValueKind::PointerDifference { .. }
            | ValueKind::VaArg { .. }
            | ValueKind::Capture { .. }
            | ValueKind::VaStart { .. }
            | ValueKind::VaEnd { .. }
            | ValueKind::VaCopy { .. }
            | ValueKind::Overflow { .. }
            | ValueKind::FloatClass { .. }
            | ValueKind::Lane { .. }
            | ValueKind::Shuffle { .. }
            | ValueKind::Intrinsic { .. }
            | ValueKind::Call {
                callee: c::Callee::Indirect(_),
                ..
            }) => bail!("unsupported expression: {unsupported:?}"),
        }
    }
}
