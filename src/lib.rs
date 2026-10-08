use anyhow::{Context, Result, anyhow, ensure};
use cranelift_codegen::{
    ir::{self as clif, AbiParam, InstBuilder, Signature, types},
    isa::CallConv,
    settings, verify_function,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use slate_parser::{
    compiler_args::CompilerFlavor,
    dialect::Dialect,
    files::SearchPaths,
    ir::{self as slate, Number, NumericType, Parameters, Statement, Type, ValueKind},
    parser::Parser,
    sema::Sema,
    target_info::TargetInfo,
};

#[cfg(feature = "native")]
pub mod native;

pub fn parse(source: &str, target: &str) -> Result<slate::Module> {
    let target = TargetInfo::for_triple(target)?;
    let dialect = Dialect::for_flavor(CompilerFlavor::Clang, target);
    let mut parser = Parser::new(SearchPaths::default(), dialect);
    let (unit, files) = parser
        .parse_file_with_source(std::path::Path::new("input.c"), source.to_owned())
        .map_err(|e| anyhow!("{e}"))?;
    let mut sema = Sema::new(&unit);
    sema.names().map_err(|e| anyhow!("{e}"))?;
    sema.analyze(&files).map_err(frontend_errors)?;
    let (module, _) = sema.lower(&files).map_err(frontend_errors)?;
    Ok(module)
}

fn frontend_errors(errors: slate_parser::sema::SemaErrors) -> anyhow::Error {
    anyhow!(
        "{}",
        errors
            .errors
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    )
}

pub fn lower(module: &slate::Module, call_conv: CallConv) -> Result<clif::Function> {
    ensure!(
        module.globals.is_empty() && module.asm.is_empty() && module.types.is_empty(),
        "only a single main function returning an integer constant is supported"
    );
    ensure!(
        module.functions.len() == 1,
        "expected a single main function"
    );
    let function = &module.functions[0].value;
    ensure!(function.name == "main", "expected a main function");
    ensure!(
        matches!(&function.parameters, Parameters::Prototype { fixed, variadic: false } if fixed.is_empty()),
        "main must take no parameters (use main(void))"
    );
    ensure!(
        function.return_type.as_ref().is_some_and(is_i32),
        "main must return a signed 32-bit int"
    );
    ensure!(
        function.symbol == slate::SymbolAttributes::default()
            && function.semantics.target.is_empty()
            && !function.semantics.naked,
        "function attributes are not supported"
    );
    let body = function.body.as_ref().context("main has no definition")?;
    ensure!(body.len() == 1, "expected one return statement");
    let Statement::Return(Some(value)) = &body[0].value else {
        return Err(anyhow!("expected return of an integer constant"));
    };
    ensure!(
        is_i32(&value.ty),
        "return value must be a signed 32-bit int"
    );
    let constant: i32 = match &value.node.value {
        ValueKind::Constant(Number::Integer(number)) => number.to_string().parse()?,
        ValueKind::Constant(Number::SignedInteger(number)) => number.to_string().parse()?,
        _ => return Err(anyhow!("only integer constants are supported")),
    };

    let mut signature = Signature::new(call_conv);
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
    builder.finalize();
    verify_function(&function, &settings::Flags::new(settings::builder()))
        .map_err(|e| anyhow!("invalid CLIF: {e}"))?;
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

#[cfg_attr(feature = "web", wasm_bindgen::prelude::wasm_bindgen)]
pub struct WasmCompilation {
    wasm: Vec<u8>,
    clif: String,
}

#[cfg_attr(feature = "web", wasm_bindgen::prelude::wasm_bindgen)]
impl WasmCompilation {
    #[cfg_attr(feature = "web", wasm_bindgen::prelude::wasm_bindgen(getter))]
    pub fn wasm(&self) -> Vec<u8> {
        self.wasm.clone()
    }

    #[cfg_attr(feature = "web", wasm_bindgen::prelude::wasm_bindgen(getter))]
    pub fn clif(&self) -> String {
        self.clif.clone()
    }
}

pub fn compile_program(source: &str) -> Result<WasmCompilation> {
    let module = parse(source, "i686-unknown-linux-gnu")?;
    let function = lower(&module, CallConv::SystemV)?;
    let wasm =
        clif2wasm::translate_module(&[clif2wasm::NamedClifFunc::new("main", &function)], "main")?;
    Ok(WasmCompilation {
        wasm,
        clif: function.display().to_string(),
    })
}

pub fn compile_wasm(source: &str) -> Result<Vec<u8>> {
    Ok(compile_program(source)?.wasm)
}

#[cfg(feature = "web")]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = compileWasm)]
pub fn compile_wasm_js(source: &str) -> std::result::Result<Vec<u8>, wasm_bindgen::JsValue> {
    compile_wasm(source).map_err(|e| wasm_bindgen::JsValue::from_str(&format!("{e:#}")))
}

#[cfg(feature = "web")]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = compileProgram)]
pub fn compile_program_js(
    source: &str,
) -> std::result::Result<WasmCompilation, wasm_bindgen::JsValue> {
    compile_program(source).map_err(|e| wasm_bindgen::JsValue::from_str(&format!("{e:#}")))
}
