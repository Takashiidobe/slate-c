use super::{FunctionLowerer, LoweredValue};
use crate::diagnostics::{CompilerError, Result};
use cranelift_codegen::ir::{self as clif, InstBuilder, MemFlagsData, types};
use cranelift_module::Module;
use slate_parser::ir::{self as c, Type, ValueKind};

impl<M: Module> FunctionLowerer<'_, '_, '_, '_, M> {
    pub(super) fn scalar_value(&mut self, value: &c::Value) -> Result<clif::Value> {
        self.value(value)?
            .scalar()
            .map_err(|error| error.at(value.node.expansion))
    }

    pub(super) fn value(&mut self, value: &c::Value) -> Result<LoweredValue> {
        self.value_inner(value)
            .map_err(|error| error.at(value.node.expansion))
    }

    fn value_inner(&mut self, value: &c::Value) -> Result<LoweredValue> {
        match &value.node.value {
            ValueKind::Void => Ok(LoweredValue::Void),
            ValueKind::Copy { operand, .. } => {
                let result = self.value(operand)?;
                if *self.module.resolved(&value.ty)? == Type::Void {
                    Ok(LoweredValue::Void)
                } else {
                    Ok(result)
                }
            }
            ValueKind::Read { place, ordering } => {
                if ordering.is_some() {
                    return Err(CompilerError::Unsupported(
                        "atomic loads are not supported".to_string(),
                    ));
                };
                let address = self.place(place)?;
                if self.module.aggregate(&value.ty)? {
                    let snapshot = self.temporary(&value.ty)?;
                    self.copy_object(snapshot, address, &value.ty)?;
                    Ok(LoweredValue::Aggregate { address: snapshot })
                } else {
                    let result = self.builder.ins().load(
                        self.module.scalar(&value.ty)?,
                        MemFlagsData::new(),
                        address,
                        0,
                    );
                    Ok(LoweredValue::Scalar(result))
                }
            }
            ValueKind::Convert { operand, .. } => {
                if *self.module.resolved(&value.ty)? == Type::Void {
                    self.value(operand)?;
                    return Ok(LoweredValue::Void);
                }
                if *self.module.resolved(&value.ty)? == Type::Bool {
                    return self.condition(operand).map(LoweredValue::Scalar);
                }
                let input = self.scalar_value(operand)?;
                let to = self.module.scalar(&value.ty)?;
                let output = if self.builder.func.dfg.value_type(input).is_float() && to.is_int() {
                    if self.module.signed(&value.ty) {
                        self.builder.ins().fcvt_to_sint(to, input)
                    } else {
                        self.builder.ins().fcvt_to_uint(to, input)
                    }
                } else {
                    self.cast(input, &operand.ty, to)?
                };
                Ok(LoweredValue::Scalar(output))
            }
            ValueKind::Store {
                place,
                value: operand,
                ordering,
            } => {
                if ordering.is_some() {
                    return Err(CompilerError::Unsupported(
                        "atomic stores are not supported".to_string(),
                    ));
                };
                let pointer = self.place(place)?;
                let output = self.value(operand)?;
                self.write_value(pointer, &place.ty, output)?;
                Ok(output)
            }
            ValueKind::Update {
                place,
                computation,
                postfix,
                ordering,
            } => {
                if ordering.is_some() {
                    return Err(CompilerError::Unsupported(
                        "atomic updates are not supported".to_string(),
                    ));
                };
                let pointer = self.place(place)?;
                let before = self.builder.ins().load(
                    self.module.scalar(&place.ty)?,
                    MemFlagsData::new(),
                    pointer,
                    0,
                );
                let saved = self.old.replace(before);
                let after = self.scalar_value(computation)?;
                self.old = saved;
                self.write_value(pointer, &place.ty, LoweredValue::Scalar(after))?;
                Ok(LoweredValue::Scalar(if *postfix { before } else { after }))
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
            } => self.conditional(condition, then_value, else_value, &value.ty),
            ValueKind::Call {
                callee: c::Callee::Direct(binding),
                signature,
                abi,
                arguments,
            } => self.call(*binding, signature, abi, arguments, &value.ty),
            ValueKind::Constant(_)
            | ValueKind::Null
            | ValueKind::AddressOf(_)
            | ValueKind::ArrayDecay { .. }
            | ValueKind::OldValue
            | ValueKind::Logical { .. }
            | ValueKind::Arith { .. }
            | ValueKind::Unary { .. }
            | ValueKind::Compare { .. }
            | ValueKind::PointerOffset { .. } => {
                self.scalar_expression(value).map(LoweredValue::Scalar)
            }
            ValueKind::LabelAddress(..) => Err(CompilerError::Unsupported(
                "label addresses are not supported".into(),
            )),
            ValueKind::Seh(..) => Err(CompilerError::Unsupported(
                "SEH expressions are not supported".into(),
            )),
            ValueKind::CodeUnits(..) => Err(CompilerError::Unsupported(
                "string values are not supported".into(),
            )),
            ValueKind::Aggregate { .. } => {
                let address = self.temporary(&value.ty)?;
                self.initialize(address, &value.ty, value)?;
                if self.module.aggregate(&value.ty)? {
                    Ok(LoweredValue::Aggregate { address })
                } else {
                    let result = self.builder.ins().load(
                        self.module.scalar(&value.ty)?,
                        MemFlagsData::new(),
                        address,
                        0,
                    );
                    Ok(LoweredValue::Scalar(result))
                }
            }
            ValueKind::FunctionDecay { .. } => Err(CompilerError::Unsupported(
                "function addresses are not supported".into(),
            )),
            ValueKind::CompareExchange { .. } => Err(CompilerError::Unsupported(
                "atomic compare-exchange are not supported".into(),
            )),
            ValueKind::Fence { .. } => Err(CompilerError::Unsupported(
                "atomic fences are not supported".into(),
            )),
            ValueKind::PointerDifference { .. } => Err(CompilerError::Unsupported(
                "pointer subtraction are not supported".into(),
            )),
            ValueKind::VaArg { .. } => Err(CompilerError::Unsupported(
                "va_arg are not supported".into(),
            )),
            ValueKind::Capture { .. } => Err(CompilerError::Unsupported(
                "captured array extents are not supported".into(),
            )),
            ValueKind::VaStart { .. } => Err(CompilerError::Unsupported(
                "va_start are not supported".into(),
            )),
            ValueKind::VaEnd { .. } => Err(CompilerError::Unsupported(
                "va_end are not supported".into(),
            )),
            ValueKind::VaCopy { .. } => Err(CompilerError::Unsupported(
                "va_copy are not supported".into(),
            )),
            ValueKind::Overflow { .. } => Err(CompilerError::Unsupported(
                "overflow builtins are not supported".into(),
            )),
            ValueKind::FloatClass { .. } => Err(CompilerError::Unsupported(
                "floating-point classification are not supported".into(),
            )),
            ValueKind::Lane { .. } => Err(CompilerError::Unsupported(
                "vector lanes are not supported".into(),
            )),
            ValueKind::Shuffle { .. } => Err(CompilerError::Unsupported(
                "vector shuffles are not supported".into(),
            )),
            ValueKind::Intrinsic { .. } => Err(CompilerError::Unsupported(
                "intrinsic expressions are not supported".into(),
            )),
            ValueKind::Call {
                callee: c::Callee::Indirect(_),
                ..
            } => Err(CompilerError::Unsupported(
                "indirect calls are not supported".into(),
            )),
        }
    }

    fn scalar_expression(&mut self, value: &c::Value) -> Result<clif::Value> {
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
                    _ => {
                        return Err(CompilerError::Unsupported(format!(
                            "unsupported constant: {number:?}"
                        )));
                    }
                })
            }
            ValueKind::Null => Ok(self.builder.ins().iconst(self.pointer, 0)),
            ValueKind::AddressOf(place) | ValueKind::ArrayDecay { place, .. } => self.place(place),
            ValueKind::OldValue => self
                .old
                .ok_or_else(|| CompilerError::InvalidIr("old value outside update".into())),
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
                self.cast(boolean, &Type::Bool, self.module.scalar(&value.ty)?)
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
                let base = self.scalar_value(pointer)?;
                let offset = self.scalar_value(amount)?;
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
            _ => Err(CompilerError::InvalidIr(format!(
                "expected scalar expression: {value}"
            ))),
        }
    }

    fn conditional(
        &mut self,
        condition: &c::Value,
        then_value: &c::Value,
        else_value: &c::Value,
        ty: &Type,
    ) -> Result<LoweredValue> {
        let condition = self.condition(condition)?;
        let yes = self.builder.create_block();
        let no = self.builder.create_block();
        let merge = self.builder.create_block();
        let void = *self.module.resolved(ty)? == Type::Void;
        let aggregate = self.module.aggregate(ty)?;
        if !void {
            let parameter = if aggregate {
                self.pointer
            } else {
                self.module.scalar(ty)?
            };
            self.builder.append_block_param(merge, parameter);
        }
        self.builder.ins().brif(condition, yes, &[], no, &[]);
        for (block, branch) in [(yes, then_value), (no, else_value)] {
            self.builder.switch_to_block(block);
            let output = self.value(branch)?;
            let argument = match output {
                LoweredValue::Void if void => None,
                LoweredValue::Scalar(value) if !void && !aggregate => Some(value),
                LoweredValue::Aggregate { address } if aggregate => Some(address),
                _ => {
                    return Err(CompilerError::InvalidIr(
                        "conditional branch value does not match result type".to_string(),
                    ));
                }
            };
            let arguments = argument.into_iter().map(Into::into).collect::<Vec<_>>();
            self.builder.ins().jump(merge, &arguments);
        }
        self.builder.switch_to_block(merge);
        if void {
            Ok(LoweredValue::Void)
        } else if aggregate {
            Ok(LoweredValue::Aggregate {
                address: self.builder.block_params(merge)[0],
            })
        } else {
            Ok(LoweredValue::Scalar(self.builder.block_params(merge)[0]))
        }
    }
}
