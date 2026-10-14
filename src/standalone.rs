use crate::diagnostics::{CompilerError, Result};
use cranelift_codegen::{
    ir::{self as clif, AbiParam, InstBuilder, Signature, types},
    isa::TargetFrontendConfig,
    settings, verify_function,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use slate_parser::ir::{
    self as slate, Number, NumericType, Parameters, Statement, Type, ValueKind,
};

pub fn lower(
    module: &slate::Module,
    frontend_config: TargetFrontendConfig,
) -> Result<clif::Function> {
    if !(module.globals.is_empty() && module.asm.is_empty() && module.types.is_empty()) {
        return Err(CompilerError::Unsupported(
            "only a single main function returning an integer constant is supported".to_string(),
        ));
    };
    if module.functions.len() != 1 {
        return Err(CompilerError::Unsupported(
            "expected a single main function".to_string(),
        ));
    };
    let function = &module.functions[0].value;
    if function.name != "main" {
        return Err(CompilerError::Unsupported(
            "expected a main function".to_string(),
        ));
    };
    if !(matches!(&function.parameters, Parameters::Prototype { fixed, variadic: false } if fixed.is_empty()))
    {
        return Err(CompilerError::Unsupported(
            "main must take no parameters (use main(void))".to_string(),
        ));
    };
    if !function.return_type.as_ref().is_some_and(is_i32) {
        return Err(CompilerError::Unsupported(
            "main must return a signed 32-bit int".to_string(),
        ));
    };
    if !(function.symbol == slate::SymbolAttributes::default()
        && function.semantics.target.is_empty()
        && !function.semantics.naked)
    {
        return Err(CompilerError::Unsupported(
            "function attributes are not supported".to_string(),
        ));
    };
    let body = function
        .body
        .as_ref()
        .ok_or_else(|| CompilerError::InvalidIr("main has no definition".into()))?;
    if body.len() != 1 {
        return Err(CompilerError::Unsupported(
            "expected one return statement".to_string(),
        ));
    };
    let Statement::Return(Some(value)) = &body[0].value else {
        return Err(CompilerError::Unsupported(
            "expected return of an integer constant".to_string(),
        ));
    };
    if !is_i32(&value.ty) {
        return Err(CompilerError::Unsupported(
            "return value must be a signed 32-bit int".to_string(),
        ));
    };
    let constant: i32 = match &value.node.value {
        ValueKind::Constant(Number::Integer(number)) => number.to_string().parse()?,
        ValueKind::Constant(Number::SignedInteger(number)) => number.to_string().parse()?,
        _ => {
            return Err(CompilerError::Unsupported(
                "only integer constants are supported".to_string(),
            ));
        }
    };

    let mut signature = Signature::new(frontend_config.default_call_conv);
    signature.returns.push(AbiParam::new(types::I32));
    let mut function =
        clif::Function::with_name_signature(clif::UserFuncName::user(0, 0), signature);
    let mut context = FunctionBuilderContext::new();
    let mut builder = FunctionBuilder::new(&mut function, &mut context);
    let entry = builder.create_block();
    builder.switch_to_block(entry);
    builder.seal_block(entry);
    let value = builder.ins().iconst(types::I32, i64::from(constant));
    builder.ins().return_(&[value]);
    builder.finalize(frontend_config);
    verify_function(&function, &settings::Flags::new(settings::builder()))
        .map_err(|e| CompilerError::Unsupported(format!("invalid CLIF: {e}")))?;
    Ok(function)
}

fn is_i32(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Numeric(NumericType::Integer {
            width: 32,
            signed: true,
            bit_precise: false
        })
    )
}
