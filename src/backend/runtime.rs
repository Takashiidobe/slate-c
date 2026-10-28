use crate::diagnostics::{CompilerError, Result};
use slate_parser::ir::Type;

use super::ModuleLowerer;
use cranelift_module::Module;

impl<M: Module> ModuleLowerer<'_, '_, M> {
    pub(super) fn call_symbol(&self, name: &str, signature: &Type) -> Result<String> {
        let name = match name {
            "__builtin_memcpy" => "memcpy",
            "__builtin_memmove" => "memmove",
            "__builtin_memset" => "memset",
            "__builtin_memcmp" => "memcmp",
            "__builtin_sqrt" => "sqrt",
            "__builtin_sqrtf" => "sqrtf",
            "__builtin_fma" => "fma",
            "__builtin_fmaf" => "fmaf",
            "__builtin_sin" => "sin",
            "__builtin_cos" => "cos",
            name if name.starts_with("__builtin_") => {
                return Err(CompilerError::Unsupported(format!(
                    "runtime lowering for builtin {name} is not supported"
                )));
            }
            name => name,
        };
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
