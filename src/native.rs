use crate::diagnostics::{CompilerError, Result};
use cranelift_codegen::{
    isa::OwnedTargetIsa,
    settings::{self, Configurable},
};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};

fn isa() -> Result<OwnedTargetIsa> {
    let mut flags = settings::builder();
    flags.set("opt_level", "none")?;
    cranelift_native::builder()
        .map_err(|e| CompilerError::Configuration(e.into()))?
        .finish(settings::Flags::new(flags))
        .map_err(Into::into)
}

pub fn run(source: &str) -> Result<i32> {
    let builder = JITBuilder::with_isa(isa()?, default_libcall_names());
    let mut module = JITModule::new(builder);
    let parsed = crate::frontend::parse_with_sources(source, &module.isa().triple().to_string())?;
    let binding = crate::entry::main_binding(&parsed.module)
        .map_err(|error| error.with_sources(&parsed.files, source))?;
    let output = crate::backend::lower(&parsed.module, &mut module)
        .map_err(|error| error.with_sources(&parsed.files, source))?;
    let main = *output
        .functions_by_binding
        .get(&binding)
        .ok_or_else(|| CompilerError::InvalidIr("main has no emitted definition".into()))?;
    module.finalize_definitions()?;
    let address = module.get_finalized_function(main);
    // entry validation ensures the signature, and module owns the live code.
    let main: unsafe extern "C" fn() -> i32 = unsafe { std::mem::transmute(address) };
    Ok(unsafe { main() })
}

pub fn compile_object(source: &str) -> Result<Vec<u8>> {
    Ok(compile(source)?.object)
}

pub struct Compilation {
    pub object: Vec<u8>,
    pub clif: String,
}

pub fn compile(source: &str) -> Result<Compilation> {
    let builder = ObjectBuilder::new(isa()?, "slate-c", default_libcall_names())?;
    let mut module = ObjectModule::new(builder);
    let parsed = crate::frontend::parse_with_sources(source, &module.isa().triple().to_string())?;
    let output = crate::backend::lower(&parsed.module, &mut module)
        .map_err(|error| error.with_sources(&parsed.files, source))?;
    Ok(Compilation {
        object: module.finish().emit()?,
        clif: output.clif,
    })
}
