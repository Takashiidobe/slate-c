use anyhow::{Context, Result, bail};
use cranelift_codegen::ir::{self as clif, types};
use slate_parser::ir::{self as c, FloatType, NumericType, Type};

use super::ModuleLowerer;
use cranelift_module::Module;

impl<M: Module> ModuleLowerer<'_, '_, M> {
    pub(super) fn resolved<'a>(&'a self, ty: &'a Type) -> Result<&'a Type> {
        if let Type::Defined(id) = ty {
            let definition = self
                .module
                .types
                .iter()
                .find(|t| t.value.id == *id)
                .context("missing type definition")?;
            match &definition.value.kind {
                c::TypeDefinitionKind::Alias(inner) => self.resolved(inner),
                c::TypeDefinitionKind::Enum {
                    underlying: Some(inner),
                    ..
                } => self.resolved(inner),
                _ => Ok(ty),
            }
        } else {
            Ok(ty)
        }
    }

    pub(super) fn scalar(&self, ty: &Type) -> Result<clif::Type> {
        Ok(match self.resolved(ty)? {
            Type::Bool => types::I8,
            Type::Numeric(NumericType::Integer { width, .. }) => match width {
                8 => types::I8,
                16 => types::I16,
                32 => types::I32,
                64 => types::I64,
                _ => bail!("integer width {width} is not supported"),
            },
            Type::Numeric(NumericType::Float(FloatType::F32)) => types::F32,
            Type::Numeric(NumericType::Float(FloatType::F64)) => types::F64,
            Type::Pointer { .. } => types::I64,
            other => bail!("unsupported scalar type: {other}"),
        })
    }

    pub(super) fn signed(&self, ty: &Type) -> bool {
        matches!(
            self.resolved(ty),
            Ok(Type::Numeric(NumericType::Integer { signed: true, .. }))
        )
    }

    pub(super) fn size_of(&self, ty: &Type) -> Result<u64> {
        match self.resolved(ty)? {
            Type::Array {
                element,
                length: Some(length),
            } => Ok(self.size_of(element)? * length),
            Type::Defined(id) => match &self
                .module
                .types
                .iter()
                .find(|t| t.value.id == *id)
                .context("missing type")?
                .value
                .kind
            {
                c::TypeDefinitionKind::Record {
                    layout: Some(layout),
                    ..
                } => Ok(layout.size),
                _ => bail!("incomplete object type"),
            },
            other => Ok(u64::from(self.scalar(other)?.bytes())),
        }
    }
}
