mod abi;
mod data;
mod function;
mod layout;
mod runtime;
mod symbols;

#[cfg(feature = "browser")]
use anyhow::Context;
use anyhow::{Result, ensure};
use cranelift_codegen::ir as clif;
use cranelift_frontend::FunctionBuilderContext;
#[cfg(feature = "native")]
use cranelift_module::FuncId;
use cranelift_module::Module;
use slate_parser::ir as c;
#[cfg(feature = "native")]
use slate_parser::ir::BindingId;
#[cfg(feature = "native")]
use std::collections::HashMap;

use symbols::Symbols;

#[cfg(feature = "browser")]
#[derive(serde::Serialize)]
pub struct FunctionOutput {
    pub name: String,
    pub code: Vec<u8>,
}

pub struct Output {
    #[cfg(feature = "native")]
    pub functions_by_binding: HashMap<BindingId, FuncId>,
    #[cfg(feature = "browser")]
    pub clif: String,
    #[cfg(feature = "browser")]
    pub functions: Vec<FunctionOutput>,
}

pub(super) struct ModuleLowerer<'s, 'm, M: Module> {
    module: &'s c::Module,
    backend: &'m mut M,
    symbols: Symbols,
}

pub fn lower(slate: &c::Module, backend: &mut impl Module) -> Result<Output> {
    ModuleLowerer {
        module: slate,
        backend,
        symbols: Symbols::default(),
    }
    .lower()
}

impl<M: Module> ModuleLowerer<'_, '_, M> {
    fn lower(&mut self) -> Result<Output> {
        ensure!(
            self.module.asm.is_empty(),
            "global assembly is not supported"
        );
        self.declare_symbols()?;
        self.define_globals()?;
        let output = Output {
            #[cfg(feature = "native")]
            functions_by_binding: self.symbols.functions.clone(),
            #[cfg(feature = "browser")]
            clif: String::new(),
            #[cfg(feature = "browser")]
            functions: Vec::new(),
        };
        #[cfg(feature = "browser")]
        let mut output = output;
        for function in &self.module.functions {
            let function = &function.value;
            let Some(&id) = self.symbols.functions.get(&function.id) else {
                continue;
            };
            let mut context = self.backend.make_context();
            context.func.signature = self.function_signature(function)?;
            context.func.name = clif::UserFuncName::user(0, id.as_u32());
            let mut builder_context = FunctionBuilderContext::new();
            self.lower_function(function, &mut context.func, &mut builder_context)?;
            #[cfg(feature = "browser")]
            output.clif.push_str(&format!(
                "; {}\n{}\n",
                function.name,
                context.func.display()
            ));
            self.backend.define_function(id, &mut context)?;
            #[cfg(feature = "browser")]
            {
                let compiled = context.compiled_code().context("missing machine code")?;
                output.functions.push(FunctionOutput {
                    name: function.name.clone(),
                    code: compiled.buffer.data().to_vec(),
                });
            }
        }
        Ok(output)
    }
}
