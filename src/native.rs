use anyhow::{Context, Result, anyhow};
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
        .map_err(|e| anyhow!(e))?
        .finish(settings::Flags::new(flags))
        .map_err(Into::into)
}

pub fn run(source: &str) -> Result<i32> {
    let builder = JITBuilder::with_isa(isa()?, default_libcall_names());
    let mut module = JITModule::new(builder);
    let slate = crate::parse(source, &module.isa().triple().to_string())?;
    let binding = crate::entry::main_binding(&slate)?;
    let output = crate::backend::lower(&slate, &mut module)?;
    let main = *output
        .functions_by_binding
        .get(&binding)
        .context("main has no emitted definition")?;
    module.finalize_definitions()?;
    let address = module.get_finalized_function(main);
    // entry validation ensures the signature, and module owns the live code.
    let main: unsafe extern "C" fn() -> i32 = unsafe { std::mem::transmute(address) };
    Ok(unsafe { main() })
}

pub fn compile_object(source: &str) -> Result<Vec<u8>> {
    let builder = ObjectBuilder::new(isa()?, "slate-c", default_libcall_names())?;
    let mut module = ObjectModule::new(builder);
    let slate = crate::parse(source, &module.isa().triple().to_string())?;
    crate::backend::lower(&slate, &mut module)?;
    Ok(module.finish().emit()?)
}
