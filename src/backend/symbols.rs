use super::ModuleLowerer;
use anyhow::{Result, ensure};
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
                self.linkage(global.linkage)
            };
            let id = self
                .backend
                .declare_data(name, linkage, !global.variable.is_const, false)?;
            globals.insert(global.variable.id, id);
        }
        for function in &self.module.functions {
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
            let signature = self.function_signature(function)?;
            let name = function
                .symbol
                .asm_name
                .as_deref()
                .unwrap_or(&function.name);
            let id =
                self.backend
                    .declare_function(name, self.linkage(function.linkage), &signature)?;
            functions.insert(function.id, id);
        }
        self.symbols = Symbols { functions, globals };
        Ok(())
    }
}
