use crate::diagnostics::{CompilerError, Result};
use slate_parser::ir::Type;

use super::ModuleLowerer;
use cranelift_module::Module;

impl<M: Module> ModuleLowerer<'_, '_, M> {
    pub(super) fn call_symbol(&self, name: &str, signature: &Type) -> Result<String> {
        if !matches!(signature, Type::Function { variadic: true, .. }) {
            return Ok(name.to_owned());
        }
        if !matches!(
            name,
            "printf"
                | "fprintf"
                | "sprintf"
                | "snprintf"
                | "scanf"
                | "fscanf"
                | "sscanf"
                | "dprintf"
                | "asprintf"
                | "open"
                | "openat"
                | "fcntl"
                | "ioctl"
        ) {
            return Err(CompilerError::Unsupported(format!(
                "variadic calls to {name} are not supported"
            )));
        }
        Ok(format!("__slate_vararg_{name}"))
    }
}
