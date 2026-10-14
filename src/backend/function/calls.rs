use super::{FunctionLowerer, LoweredValue};
use crate::diagnostics::{CompilerError, Result};
use cranelift_codegen::ir::InstBuilder;
use cranelift_module::{Linkage, Module};
use slate_parser::ir::{self as c, BindingId, Type};

impl<M: Module> FunctionLowerer<'_, '_, '_, '_, M> {
    pub(super) fn call(
        &mut self,
        binding: BindingId,
        original: &Type,
        abi: &c::AbiSignature,
        arguments: &[c::Value],
        result_type: &Type,
    ) -> Result<LoweredValue> {
        if abi.has_nontrivial_pass() {
            return Err(CompilerError::Unsupported(
                "aggregate or non-C calling convention is not supported".to_string(),
            ));
        }
        let declaration = self
            .module
            .module
            .functions
            .iter()
            .find(|f| f.value.id == binding)
            .ok_or_else(|| CompilerError::InvalidIr("missing called function".into()))?;
        let name = declaration
            .value
            .symbol
            .asm_name
            .as_deref()
            .unwrap_or(&declaration.value.name);
        let name = self.module.call_symbol(name, original)?;
        let signature = self
            .module
            .scalar_signature(arguments.iter().map(|argument| &argument.ty), result_type)?;
        let values = arguments
            .iter()
            .map(|argument| self.scalar_value(argument))
            .collect::<Result<Vec<_>>>()?;
        let id = if let Some(id) = self.module.symbols.functions.get(&binding) {
            *id
        } else {
            self.module.backend.declare_function(
                &name,
                Linkage::Import,
                &self.module.backend.make_signature(),
            )?
        };
        let reference = self
            .module
            .backend
            .declare_func_in_func(id, self.builder.func);
        let imported = self.builder.func.import_signature(signature);
        self.builder.func.dfg.ext_funcs[reference].signature = imported;
        let call = self.builder.ins().call(reference, &values);
        Ok(match self.builder.inst_results(call).first() {
            Some(value) => LoweredValue::Scalar(*value),
            None => LoweredValue::Void,
        })
    }
}
