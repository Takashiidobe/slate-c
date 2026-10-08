use anyhow::{Result, anyhow};
use cranelift_codegen::{
    isa::OwnedTargetIsa,
    settings::{self, Configurable},
};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};

fn isa() -> Result<OwnedTargetIsa> {
    let mut flags = settings::builder();
    flags.set("opt_level", "none")?;
    cranelift_native::builder()
        .map_err(|e| anyhow!(e))?
        .finish(settings::Flags::new(flags))
        .map_err(Into::into)
}

fn define(source: &str, module: &mut impl Module) -> Result<cranelift_module::FuncId> {
    let slate = crate::parse(source, &module.isa().triple().to_string())?;
    let function = crate::lower(&slate, module.isa().frontend_config())?;
    let id = module.declare_function("main", Linkage::Export, &function.signature)?;
    let mut context = module.make_context();
    context.func = function;
    module.define_function(id, &mut context)?;
    Ok(id)
}

pub fn run(source: &str) -> Result<i32> {
    let builder = JITBuilder::with_isa(isa()?, default_libcall_names());
    let mut module = JITModule::new(builder);
    let main = define(source, &mut module)?;
    module.finalize_definitions()?;
    let address = module.get_finalized_function(main);
    // lower enforces the C () -> i32 signature, and module owns the live code.
    let main: unsafe extern "C" fn() -> i32 = unsafe { std::mem::transmute(address) };
    Ok(unsafe { main() })
}

pub fn compile_object(source: &str) -> Result<Vec<u8>> {
    let builder = ObjectBuilder::new(isa()?, "slate-c", default_libcall_names())?;
    let mut module = ObjectModule::new(builder);
    define(source, &mut module)?;
    Ok(module.finish().emit()?)
}
