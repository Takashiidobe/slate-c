use super::ModuleLowerer;
use crate::diagnostics::{CompilerError, Result};
use cranelift_module::{DataId, FuncId, Linkage, Module};
use slate_parser::ir::{self as c, BindingId};
use std::collections::HashMap;

#[derive(Default)]
pub(super) struct Symbols {
    pub(super) functions: HashMap<BindingId, FuncId>,
    pub(super) globals: HashMap<BindingId, DataId>,
}

impl<M: Module> ModuleLowerer<'_, '_, M> {
    pub(super) fn linkage(&self, value: c::Linkage) -> Linkage {
        match value {
            c::Linkage::Internal => Linkage::Local,
            c::Linkage::External => Linkage::Export,
        }
    }

    pub(super) fn declare_symbols(&mut self) -> Result<()> {
        let mut functions = HashMap::new();
        let mut globals = HashMap::new();
        for global in &self.module.globals {
            let location = global.expansion;
            (|| -> Result<()> {
                let global = &global.value;
                if global.variable.storage == c::StorageDuration::Thread {
                    return Err(CompilerError::Unsupported(
                        "thread-local storage is not supported".to_string(),
                    ));
                }
                let name = global
                    .symbol
                    .asm_name
                    .as_deref()
                    .unwrap_or(&global.variable.name);
                let linkage = if !global.definition {
                    Linkage::Import
                } else {
                    self.linkage(global.linkage)
                };
                let id =
                    self.backend
                        .declare_data(name, linkage, !global.variable.is_const, false)?;
                globals.insert(global.variable.id, id);

                Ok(())
            })()
            .map_err(|error| error.at(location))?;
        }
        for function in &self.module.functions {
            if function.value.body.is_none() || function.value.semantics.inline_only {
                continue;
            }
            let location = function.expansion;
            (|| -> Result<()> {
                let function = &function.value;
                if function.semantics.naked || !function.semantics.target.is_empty() {
                    return Err(CompilerError::Unsupported(
                        "target function attributes are not supported".to_string(),
                    ));
                }
                if function.symbol.alias.is_some() || function.symbol.ifunc.is_some() {
                    return Err(CompilerError::Unsupported(
                        "function aliases are not supported".to_string(),
                    ));
                }
                let signature = self.function_signature(function)?;
                let name = function
                    .symbol
                    .asm_name
                    .as_deref()
                    .unwrap_or(&function.name);
                let id = self.backend.declare_function(
                    name,
                    self.linkage(function.linkage),
                    &signature,
                )?;
                functions.insert(function.id, id);

                Ok(())
            })()
            .map_err(|error| error.at(location))?;
        }
        self.symbols = Symbols { functions, globals };
        Ok(())
    }
}
