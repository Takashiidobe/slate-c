use super::ModuleLowerer;
use anyhow::{Result, bail, ensure};
use cranelift_codegen::ir::{AbiParam, Signature};
use cranelift_module::Module;
use slate_parser::ir::{self as c, Type};

impl<M: Module> ModuleLowerer<'_, '_, M> {
    pub(super) fn function_signature(&self, function: &c::Function) -> Result<Signature> {
        let c::Parameters::Prototype { fixed, variadic } = &function.parameters else {
            bail!("unprototyped function definitions are not supported");
        };
        ensure!(!variadic, "variadic function definitions are not supported");
        self.scalar_signature(
            fixed.iter().map(|parameter| &parameter.value.ty),
            function.return_type.as_ref().unwrap_or(&Type::Void),
        )
    }

    pub(super) fn scalar_signature<'a>(
        &self,
        parameters: impl IntoIterator<Item = &'a Type>,
        result: &Type,
    ) -> Result<Signature> {
        let mut signature = self.backend.make_signature();
        for ty in parameters {
            signature.params.push(AbiParam::new(self.scalar(ty)?));
        }
        if *result != Type::Void {
            signature.returns.push(AbiParam::new(self.scalar(result)?));
        }
        Ok(signature)
    }
}
