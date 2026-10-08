use std::collections::HashMap;

use anyhow::{Context, Result, bail, ensure};
use cranelift_codegen::ir::{
    self as clif, AbiParam, InstBuilder, MemFlagsData, Signature, StackSlotData, StackSlotKind,
    types,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{DataDescription, DataId, FuncId, Linkage, Module};
use cranelift_object::ObjectModule;
use serde::Serialize;
use slate_parser::{
    ast::Span,
    ir::{self as c, BindingId, FloatType, NumericType, PlaceKind, Statement, Type, ValueKind},
};

#[derive(Serialize)]
pub struct FunctionOutput {
    pub name: String,
    pub code: Vec<u8>,
}

pub struct Output {
    pub clif: String,
    pub functions: Vec<FunctionOutput>,
    pub entry: String,
}

pub fn lower(slate: &c::Module, module: &mut ObjectModule) -> Result<Output> {
    ensure!(slate.asm.is_empty(), "global assembly is not supported");
    let mut functions = HashMap::new();
    let mut globals = HashMap::new();
    for global in &slate.globals {
        let global = &global.value;
        ensure!(
            global.variable.storage != c::StorageDuration::Thread,
            "thread-local storage is not supported"
        );
        let name = global
            .symbol
            .asm_name
            .as_deref()
            .unwrap_or(&global.variable.name);
        let linkage = if !global.definition {
            Linkage::Import
        } else {
            linkage(global.linkage)
        };
        let id = module.declare_data(name, linkage, !global.variable.is_const, false)?;
        globals.insert(global.variable.id, id);
        if global.definition {
            ensure!(
                global.symbol.alias.is_none() && global.symbol.ifunc.is_none(),
                "global aliases are not supported"
            );
            let size = size_of(&global.variable.ty, slate)?;
            let mut data = DataDescription::new();
            data.set_align(
                global
                    .variable
                    .alignment
                    .unwrap_or(size.min(8).next_power_of_two()),
            );
            if let Some(value) = &global.variable.initializer {
                data.define(initial_bytes(value, size)?.into_boxed_slice());
            } else {
                data.define_zeroinit(size as usize);
            }
            module.define_data(id, &data)?;
        }
    }
    for function in &slate.functions {
        let function = &function.value;
        if function.body.is_none() || function.semantics.inline_only {
            continue;
        }
        ensure!(
            !function.semantics.naked && function.semantics.target.is_empty(),
            "target function attributes are not supported"
        );
        ensure!(
            function.symbol.alias.is_none() && function.symbol.ifunc.is_none(),
            "function aliases are not supported"
        );
        let signature = function_signature(function, slate, module)?;
        let name = function
            .symbol
            .asm_name
            .as_deref()
            .unwrap_or(&function.name);
        let id = module.declare_function(name, linkage(function.linkage), &signature)?;
        functions.insert(function.id, id);
    }
    ensure!(
        slate
            .functions
            .iter()
            .any(|f| f.value.name == "main" && f.value.body.is_some()),
        "expected a main definition"
    );
    let mut output = Output {
        clif: String::new(),
        functions: Vec::new(),
        entry: String::new(),
    };
    for function in &slate.functions {
        let function = &function.value;
        let Some(&id) = functions.get(&function.id) else {
            continue;
        };
        let mut context = module.make_context();
        context.func.signature = function_signature(function, slate, module)?;
        context.func.name = clif::UserFuncName::user(0, id.as_u32());
        let mut builder_context = FunctionBuilderContext::new();
        let config = module.isa().frontend_config();
        {
            let mut builder = FunctionBuilder::new(&mut context.func, &mut builder_context);
            let entry = builder.create_block();
            builder.append_block_params_for_function_params(entry);
            builder.switch_to_block(entry);
            let mut emitter = Emitter {
                builder,
                module,
                slate,
                functions: &functions,
                globals: &globals,
                locals: HashMap::new(),
                loops: HashMap::new(),
                pointer: config.pointer_type(),
                old: None,
            };
            if let c::Parameters::Prototype { fixed, .. } = &function.parameters {
                let parameters = emitter.builder.block_params(entry).to_vec();
                for (parameter, value) in fixed.iter().zip(parameters) {
                    let slot = emitter.local(parameter.value.id, &parameter.value.ty)?;
                    emitter
                        .builder
                        .ins()
                        .store(MemFlagsData::new(), value, slot, 0);
                }
            }
            emitter.statements(function.body.as_ref().context("missing function body")?)?;
            if !emitter.terminated() {
                match &function.fallthrough {
                    Some(c::Fallthrough::ReturnZero) => {
                        let value = emitter.builder.ins().iconst(types::I32, 0);
                        emitter.builder.ins().return_(&[value]);
                    }
                    Some(c::Fallthrough::Return(value)) => {
                        let value = emitter.value(value)?;
                        emitter.builder.ins().return_(&[value]);
                    }
                    Some(c::Fallthrough::ReturnVoid) => {
                        emitter.builder.ins().return_(&[]);
                    }
                    _ if function.name == "main" => {
                        let value = emitter.builder.ins().iconst(types::I32, 0);
                        emitter.builder.ins().return_(&[value]);
                    }
                    _ => {
                        emitter.builder.ins().trap(clif::TrapCode::unwrap_user(1));
                    }
                }
            }
            emitter.builder.seal_all_blocks();
            emitter.builder.finalize(config);
        }
        output.clif.push_str(&format!(
            "; {}\n{}\n",
            function.name,
            context.func.display()
        ));
        module.define_function(id, &mut context)?;
        let compiled = context.compiled_code().context("missing machine code")?;
        output.functions.push(FunctionOutput {
            name: function.name.clone(),
            code: compiled.buffer.data().to_vec(),
        });
    }
    let main = slate
        .functions
        .iter()
        .find(|f| f.value.name == "main" && f.value.body.is_some())
        .context("missing main")?;
    ensure!(
        matches!(&main.value.parameters, c::Parameters::Prototype { fixed, variadic: false } if fixed.is_empty())
            && matches!(
                main.value.return_type.as_ref(),
                Some(Type::Numeric(NumericType::Integer {
                    width: 32,
                    signed: true,
                    ..
                }))
            ),
        "bare-metal execution requires int main(void)"
    );
    let main_id = *functions
        .get(&main.value.id)
        .context("main cannot be inline-only")?;
    let mut entry_name = "__slate_run".to_owned();
    while slate
        .functions
        .iter()
        .any(|function| function.value.name == entry_name)
    {
        entry_name.push('_');
    }
    let mut signature = module.make_signature();
    signature
        .params
        .extend([AbiParam::new(types::I64), AbiParam::new(types::I64)]);
    signature.returns.push(AbiParam::new(types::I64));
    let entry_id = module.declare_function(&entry_name, Linkage::Export, &signature)?;
    let mut context = module.make_context();
    context.func.signature = signature;
    context.func.name = clif::UserFuncName::user(0, entry_id.as_u32());
    let mut builder_context = FunctionBuilderContext::new();
    let config = module.isa().frontend_config();
    {
        let main = module.declare_func_in_func(main_id, &mut context.func);
        let mut builder = FunctionBuilder::new(&mut context.func, &mut builder_context);
        let block = builder.create_block();
        builder.append_block_params_for_function_params(block);
        builder.switch_to_block(block);
        builder.seal_block(block);
        let result_pointer = builder.block_params(block)[1];
        let call = builder.ins().call(main, &[]);
        let result = builder.inst_results(call)[0];
        builder
            .ins()
            .store(MemFlagsData::new(), result, result_pointer, 64);
        builder.ins().return_(&[result_pointer]);
        builder.finalize(config);
    }
    module.define_function(entry_id, &mut context)?;
    output.entry = entry_name;
    Ok(output)
}

fn linkage(value: c::Linkage) -> Linkage {
    match value {
        c::Linkage::Internal => Linkage::Local,
        c::Linkage::External => Linkage::Export,
    }
}

fn resolved<'a>(ty: &'a Type, slate: &'a c::Module) -> Result<&'a Type> {
    if let Type::Defined(id) = ty {
        let definition = slate
            .types
            .iter()
            .find(|t| t.value.id == *id)
            .context("missing type definition")?;
        match &definition.value.kind {
            c::TypeDefinitionKind::Alias(inner) => resolved(inner, slate),
            c::TypeDefinitionKind::Enum {
                underlying: Some(inner),
                ..
            } => resolved(inner, slate),
            _ => Ok(ty),
        }
    } else {
        Ok(ty)
    }
}

fn scalar(ty: &Type, slate: &c::Module) -> Result<clif::Type> {
    Ok(match resolved(ty, slate)? {
        Type::Bool => types::I8,
        Type::Numeric(NumericType::Integer { width, .. }) => match width {
            8 => types::I8,
            16 => types::I16,
            32 => types::I32,
            64 => types::I64,
            _ => bail!("integer width {width} is not supported"),
        },
        Type::Numeric(NumericType::Float(FloatType::F32)) => types::F32,
        Type::Numeric(NumericType::Float(FloatType::F64)) => types::F64,
        Type::Pointer { .. } => types::I64,
        other => bail!("unsupported scalar type: {other}"),
    })
}

fn signed(ty: &Type, slate: &c::Module) -> bool {
    matches!(
        resolved(ty, slate),
        Ok(Type::Numeric(NumericType::Integer { signed: true, .. }))
    )
}

fn size_of(ty: &Type, slate: &c::Module) -> Result<u64> {
    match resolved(ty, slate)? {
        Type::Array {
            element,
            length: Some(length),
        } => Ok(size_of(element, slate)? * length),
        Type::Defined(id) => match &slate
            .types
            .iter()
            .find(|t| t.value.id == *id)
            .context("missing type")?
            .value
            .kind
        {
            c::TypeDefinitionKind::Record {
                layout: Some(layout),
                ..
            } => Ok(layout.size),
            _ => bail!("incomplete object type"),
        },
        other => Ok(u64::from(scalar(other, slate)?.bytes())),
    }
}

fn initial_bytes(value: &c::Value, size: u64) -> Result<Vec<u8>> {
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
                bail!("expected string array");
            };
            let width = match element.as_ref() {
                Type::Numeric(NumericType::Integer { width, .. }) => width / 8,
                _ => bail!("unsupported string encoding"),
            };
            let mut bytes = Vec::new();
            for unit in units {
                bytes.extend_from_slice(&unit.to_le_bytes()[..width as usize]);
            }
            bytes.resize(size as usize, 0);
            Ok(bytes)
        }
        ValueKind::Copy { operand, .. } | ValueKind::Convert { operand, .. } => {
            initial_bytes(operand, size)
        }
        _ => bail!("global initializer is not supported: {value}"),
    }
}

fn function_signature(
    function: &c::Function,
    slate: &c::Module,
    module: &ObjectModule,
) -> Result<Signature> {
    let mut signature = module.make_signature();
    let c::Parameters::Prototype { fixed, variadic } = &function.parameters else {
        bail!("unprototyped function definitions are not supported");
    };
    ensure!(!variadic, "variadic function definitions are not supported");
    for parameter in fixed {
        signature
            .params
            .push(AbiParam::new(scalar(&parameter.value.ty, slate)?));
    }
    if let Some(ty) = &function.return_type
        && *ty != Type::Void
    {
        signature.returns.push(AbiParam::new(scalar(ty, slate)?));
    }
    Ok(signature)
}

struct Emitter<'a, 'b> {
    builder: FunctionBuilder<'a>,
    module: &'b mut ObjectModule,
    slate: &'b c::Module,
    functions: &'b HashMap<BindingId, FuncId>,
    globals: &'b HashMap<BindingId, DataId>,
    locals: HashMap<BindingId, clif::StackSlot>,
    loops: HashMap<BindingId, (clif::Block, clif::Block)>,
    pointer: clif::Type,
    old: Option<clif::Value>,
}

impl Emitter<'_, '_> {
    fn terminated(&self) -> bool {
        self.builder
            .current_block()
            .and_then(|block| self.builder.func.layout.last_inst(block))
            .is_some_and(|inst| self.builder.func.dfg.insts[inst].opcode().is_terminator())
    }

    fn local(&mut self, id: BindingId, ty: &Type) -> Result<clif::Value> {
        let slot = if let Some(slot) = self.locals.get(&id) {
            *slot
        } else {
            let size = u32::try_from(size_of(ty, self.slate)?)?;
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

    fn place(&mut self, place: &c::Place) -> Result<clif::Value> {
        ensure!(
            !place.access.atomic && !place.access.volatile,
            "atomic and volatile access is not supported"
        );
        match &place.kind {
            PlaceKind::Binding(id) => {
                if let Some(data) = self.globals.get(id) {
                    let reference = self.module.declare_data_in_func(*data, self.builder.func);
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
                    .imul_imm_s(index_value, size_of(&place.ty, self.slate)? as i64);
                Ok(self.builder.ins().iadd(base, offset))
            }
            PlaceKind::CompoundLiteral {
                object,
                initializer,
                storage,
                ..
            } => {
                if *storage == c::StorageDuration::Static {
                    let size = size_of(&place.ty, self.slate)?;
                    let data = self.module.declare_anonymous_data(false, false)?;
                    let mut description = DataDescription::new();
                    description.set_align(8);
                    description.define(initial_bytes(initializer, size)?.into_boxed_slice());
                    self.module.define_data(data, &description)?;
                    let reference = self.module.declare_data_in_func(data, self.builder.func);
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
                    .slate
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

    fn initialize(
        &mut self,
        pointer: clif::Value,
        ty: &Type,
        initializer: &c::Value,
    ) -> Result<()> {
        if matches!(resolved(ty, self.slate)?, Type::Array { .. }) {
            let bytes = initial_bytes(initializer, size_of(ty, self.slate)?)?;
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

    fn cast(&mut self, value: clif::Value, from: &Type, to: clif::Type) -> Result<clif::Value> {
        let old = self.builder.func.dfg.value_type(value);
        if old == to {
            return Ok(value);
        }
        Ok(if old.is_int() && to.is_int() {
            if old.bits() > to.bits() {
                self.builder.ins().ireduce(to, value)
            } else if signed(from, self.slate) {
                self.builder.ins().sextend(to, value)
            } else {
                self.builder.ins().uextend(to, value)
            }
        } else if old.is_int() && to.is_float() {
            if signed(from, self.slate) {
                self.builder.ins().fcvt_from_sint(to, value)
            } else {
                self.builder.ins().fcvt_from_uint(to, value)
            }
        } else if old.is_float() && to.is_int() {
            if signed(from, self.slate) {
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

    fn condition(&mut self, value: &c::Value) -> Result<clif::Value> {
        let value = self.value(value)?;
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

    fn value(&mut self, value: &c::Value) -> Result<clif::Value> {
        match &value.node.value {
            ValueKind::Constant(number) => {
                let ty = scalar(&value.ty, self.slate)?;
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
                    scalar(&value.ty, self.slate)?,
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
                let to = scalar(&value.ty, self.slate)?;
                if self.builder.func.dfg.value_type(input).is_float() && to.is_int() {
                    return Ok(if signed(&value.ty, self.slate) {
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
                    scalar(&place.ty, self.slate)?,
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
                    .append_block_param(merge, scalar(&value.ty, self.slate)?);
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
                    .uextend(scalar(&value.ty, self.slate)?, boolean))
            }
            ValueKind::Arith {
                op, left, right, ..
            } => {
                let lhs = self.value(left)?;
                let rhs = self.value(right)?;
                let floating = self.builder.func.dfg.value_type(lhs).is_float();
                Ok(match (op, floating) {
                    (c::ArithOp::Add, false) => self.builder.ins().iadd(lhs, rhs),
                    (c::ArithOp::Sub, false) => self.builder.ins().isub(lhs, rhs),
                    (c::ArithOp::Mul, false) => self.builder.ins().imul(lhs, rhs),
                    (c::ArithOp::Div, false) if signed(&left.ty, self.slate) => {
                        self.builder.ins().sdiv(lhs, rhs)
                    }
                    (c::ArithOp::Div, false) => self.builder.ins().udiv(lhs, rhs),
                    (c::ArithOp::Rem, false) if signed(&left.ty, self.slate) => {
                        self.builder.ins().srem(lhs, rhs)
                    }
                    (c::ArithOp::Rem, false) => self.builder.ins().urem(lhs, rhs),
                    (c::ArithOp::And, false) => self.builder.ins().band(lhs, rhs),
                    (c::ArithOp::Or, false) => self.builder.ins().bor(lhs, rhs),
                    (c::ArithOp::Xor, false) => self.builder.ins().bxor(lhs, rhs),
                    (c::ArithOp::Shl, false) => self.builder.ins().ishl(lhs, rhs),
                    (c::ArithOp::Shr, false) if signed(&left.ty, self.slate) => {
                        self.builder.ins().sshr(lhs, rhs)
                    }
                    (c::ArithOp::Shr, false) => self.builder.ins().ushr(lhs, rhs),
                    (c::ArithOp::Add, true) => self.builder.ins().fadd(lhs, rhs),
                    (c::ArithOp::Sub, true) => self.builder.ins().fsub(lhs, rhs),
                    (c::ArithOp::Mul, true) => self.builder.ins().fmul(lhs, rhs),
                    (c::ArithOp::Div, true) => self.builder.ins().fdiv(lhs, rhs),
                    _ => bail!("unsupported arithmetic operation: {op}"),
                })
            }
            ValueKind::Unary { op, operand, .. } => {
                let input = self.value(operand)?;
                Ok(
                    match (op, self.builder.func.dfg.value_type(input).is_float()) {
                        (c::UnaryArithOp::Neg, true) => self.builder.ins().fneg(input),
                        (c::UnaryArithOp::Neg, false) => self.builder.ins().ineg(input),
                        (c::UnaryArithOp::Not, false) if operand.ty == Type::Bool => self
                            .builder
                            .ins()
                            .icmp_imm_s(clif::condcodes::IntCC::Equal, input, 0),
                        (c::UnaryArithOp::Not, false) => self.builder.ins().bnot(input),
                        _ => bail!("unsupported unary operation"),
                    },
                )
            }
            ValueKind::Compare {
                op, left, right, ..
            } => {
                let lhs = self.value(left)?;
                let rhs = self.value(right)?;
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
                    let code = match (op, signed(&left.ty, self.slate)) {
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
                let target = scalar(&value.ty, self.slate)?;
                Ok(if target == types::I8 {
                    boolean
                } else {
                    self.builder.ins().uextend(target, boolean)
                })
            }
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
                    .imul_imm_s(offset, size_of(element, self.slate)? as i64);
                Ok(if *subtract {
                    self.builder.ins().isub(base, offset)
                } else {
                    self.builder.ins().iadd(base, offset)
                })
            }
            ValueKind::Call {
                callee: c::Callee::Direct(binding),
                signature: original,
                abi,
                arguments,
            } => {
                ensure!(
                    !abi.has_nontrivial_pass(),
                    "aggregate or non-C calling convention is not supported"
                );
                let declaration = self
                    .slate
                    .functions
                    .iter()
                    .find(|f| f.value.id == *binding)
                    .context("missing called function")?;
                let name = declaration
                    .value
                    .symbol
                    .asm_name
                    .clone()
                    .unwrap_or_else(|| declaration.value.name.clone());
                let variadic = matches!(original, Type::Function { variadic: true, .. });
                ensure!(
                    !variadic,
                    "variadic calls require a runtime and are not supported in bare-metal execution"
                );
                let mut signature = self.module.make_signature();
                let mut values = Vec::new();
                for argument in arguments {
                    signature
                        .params
                        .push(AbiParam::new(scalar(&argument.ty, self.slate)?));
                    values.push(self.value(argument)?);
                }
                if value.ty != Type::Void {
                    signature
                        .returns
                        .push(AbiParam::new(scalar(&value.ty, self.slate)?));
                }
                let id = if let Some(id) = self.functions.get(binding) {
                    *id
                } else {
                    self.module.declare_function(
                        &name,
                        Linkage::Import,
                        &self.module.make_signature(),
                    )?
                };
                let reference = self.module.declare_func_in_func(id, self.builder.func);
                let imported = self.builder.func.import_signature(signature);
                self.builder.func.dfg.ext_funcs[reference].signature = imported;
                let call = self.builder.ins().call(reference, &values);
                Ok(self
                    .builder
                    .inst_results(call)
                    .first()
                    .copied()
                    .unwrap_or_else(|| self.builder.ins().iconst(types::I8, 0)))
            }
            other => bail!("unsupported expression: {other:?}"),
        }
    }

    fn statements(&mut self, body: &[Span<Statement>]) -> Result<()> {
        for statement in body {
            if self.terminated() {
                break;
            }
            match &statement.value {
                Statement::Comment(_) | Statement::Null => {}
                Statement::Return(value) => {
                    let values = if let Some(value) = value {
                        vec![self.value(value)?]
                    } else {
                        Vec::new()
                    };
                    self.builder.ins().return_(&values);
                }
                Statement::Expression(value) => {
                    self.value(value)?;
                }
                Statement::Block(body) => self.statements(body)?,
                Statement::Temporary {
                    id,
                    ty,
                    initializer,
                    ..
                } => {
                    let pointer = self.local(*id, ty)?;
                    if let Some(value) = initializer {
                        self.initialize(pointer, ty, value)?;
                    }
                }
                Statement::Let(variable) => {
                    ensure!(
                        variable.storage == c::StorageDuration::Automatic,
                        "static local variables are not supported"
                    );
                    ensure!(
                        variable.cleanup.is_none(),
                        "cleanup attributes are not supported"
                    );
                    let pointer = self.local(variable.id, &variable.ty)?;
                    if let Some(value) = &variable.initializer {
                        self.initialize(pointer, &variable.ty, value)?;
                    }
                }
                Statement::Write {
                    place,
                    value,
                    ordering,
                    ..
                } => {
                    ensure!(ordering.is_none(), "atomic writes are not supported");
                    let pointer = self.place(place)?;
                    let value = self.value(value)?;
                    self.builder
                        .ins()
                        .store(MemFlagsData::new(), value, pointer, 0);
                }
                Statement::If {
                    condition,
                    then_body,
                    else_body,
                } => {
                    let condition = self.condition(condition)?;
                    let yes = self.builder.create_block();
                    let no = self.builder.create_block();
                    let merge = self.builder.create_block();
                    self.builder.ins().brif(condition, yes, &[], no, &[]);
                    self.builder.switch_to_block(yes);
                    self.statements(then_body)?;
                    if !self.terminated() {
                        self.builder.ins().jump(merge, &[]);
                    }
                    self.builder.switch_to_block(no);
                    if let Some(body) = else_body {
                        self.statements(body)?;
                    }
                    if !self.terminated() {
                        self.builder.ins().jump(merge, &[]);
                    }
                    self.builder.switch_to_block(merge);
                }
                Statement::While {
                    id,
                    condition,
                    body,
                } => self.loop_body(*id, &[], Some(condition), None, body, false)?,
                Statement::DoWhile {
                    id,
                    condition,
                    body,
                } => self.loop_body(*id, &[], Some(condition), None, body, true)?,
                Statement::For {
                    id,
                    init,
                    condition,
                    increment,
                    body,
                } => self.loop_body(
                    *id,
                    init,
                    condition.as_ref(),
                    increment.as_ref(),
                    body,
                    false,
                )?,
                Statement::Break(id) | Statement::Continue(id) => {
                    let (next, end) = self.loops.get(id).context("unknown loop")?;
                    self.builder.ins().jump(
                        if matches!(statement.value, Statement::Break(_)) {
                            *end
                        } else {
                            *next
                        },
                        &[],
                    );
                }
                other => bail!("unsupported statement: {other:?}"),
            }
        }
        Ok(())
    }

    fn loop_body(
        &mut self,
        id: BindingId,
        init: &[Span<Statement>],
        condition: Option<&c::Evaluation>,
        increment: Option<&c::Evaluation>,
        body: &[Span<Statement>],
        do_while: bool,
    ) -> Result<()> {
        self.statements(init)?;
        let test = self.builder.create_block();
        let iteration = self.builder.create_block();
        let next = self.builder.create_block();
        let end = self.builder.create_block();
        self.loops.insert(id, (next, end));
        self.builder
            .ins()
            .jump(if do_while { iteration } else { test }, &[]);
        self.builder.switch_to_block(test);
        if let Some(condition) = condition {
            self.statements(&condition.statements)?;
            let condition = self.condition(&condition.value)?;
            self.builder.ins().brif(condition, iteration, &[], end, &[]);
        } else {
            self.builder.ins().jump(iteration, &[]);
        }
        self.builder.switch_to_block(iteration);
        self.statements(body)?;
        if !self.terminated() {
            self.builder.ins().jump(next, &[]);
        }
        self.builder.switch_to_block(next);
        if let Some(increment) = increment {
            self.statements(&increment.statements)?;
            self.value(&increment.value)?;
        }
        self.builder.ins().jump(test, &[]);
        self.builder.switch_to_block(end);
        self.loops.remove(&id);
        Ok(())
    }
}
