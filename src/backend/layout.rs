use crate::diagnostics::{CompilerError, Result};
use cranelift_codegen::ir::{self as clif, types};
use slate_parser::ir::{self as c, FloatType, NumericType, Type};

use super::ModuleLowerer;
use cranelift_module::Module;

#[derive(Clone, Copy)]
pub(super) struct ObjectLayout {
    pub size: u64,
    pub alignment: u64,
}

impl<M: Module> ModuleLowerer<'_, '_, M> {
    pub(super) fn resolved<'a>(&'a self, ty: &'a Type) -> Result<&'a Type> {
        if let Type::Defined(id) = ty {
            let definition = self
                .module
                .types
                .iter()
                .find(|t| t.value.id == *id)
                .ok_or_else(|| CompilerError::InvalidIr("missing type definition".into()))?;
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
                _ => {
                    return Err(CompilerError::Unsupported(format!(
                        "integer width {width} is not supported"
                    )));
                }
            },
            Type::Numeric(NumericType::Float(FloatType::F32)) => types::F32,
            Type::Numeric(NumericType::Float(FloatType::F64)) => types::F64,
            Type::Pointer { .. } => types::I64,
            other => {
                return Err(CompilerError::Unsupported(format!(
                    "unsupported scalar type: {other}"
                )));
            }
        })
    }

    pub(super) fn signed(&self, ty: &Type) -> bool {
        matches!(
            self.resolved(ty),
            Ok(Type::Numeric(NumericType::Integer { signed: true, .. }))
        )
    }

    pub(super) fn aggregate(&self, ty: &Type) -> Result<bool> {
        Ok(matches!(
            self.resolved(ty)?,
            Type::Array { .. } | Type::Defined(_)
        ))
    }

    pub(super) fn size_of(&self, ty: &Type) -> Result<u64> {
        Ok(self.layout(ty)?.size)
    }

    pub(super) fn record_layout(&self, ty: &Type) -> Result<&c::RecordLayout> {
        let Type::Defined(id) = self.resolved(ty)? else {
            return Err(CompilerError::InvalidIr("expected record type".into()));
        };
        match &self
            .module
            .types
            .iter()
            .find(|definition| definition.value.id == *id)
            .ok_or_else(|| CompilerError::InvalidIr("missing record type".into()))?
            .value
            .kind
        {
            c::TypeDefinitionKind::Record {
                layout: Some(layout),
                ..
            } => Ok(layout),
            _ => Err(CompilerError::Unsupported("incomplete record type".into())),
        }
    }

    pub(super) fn object_layout(&self, ty: &Type, alignment: Option<u64>) -> Result<ObjectLayout> {
        let mut layout = self.layout(ty)?;
        if let Some(alignment) = alignment {
            layout.alignment = alignment;
        }
        if !layout.alignment.is_power_of_two() {
            return Err(CompilerError::InvalidIr(
                "object alignment must be a nonzero power of two".into(),
            ));
        }
        Ok(layout)
    }

    fn layout(&self, ty: &Type) -> Result<ObjectLayout> {
        match self.resolved(ty)? {
            Type::Array {
                element,
                length: Some(length),
            } => {
                let element = self.layout(element)?;
                let size = element.size.checked_mul(*length).ok_or_else(|| {
                    CompilerError::Unsupported("array object size exceeds u64".into())
                })?;
                Ok(ObjectLayout {
                    size,
                    alignment: element.alignment,
                })
            }
            Type::Defined(_) => {
                let record = self.record_layout(ty)?;
                Ok(ObjectLayout {
                    size: record.size,
                    alignment: record.align,
                })
            }
            Type::Array { length: None, .. } => {
                Err(CompilerError::Unsupported("incomplete array type".into()))
            }
            other => {
                let storage = self.module.target.storage_of(other.clone())?;
                Ok(ObjectLayout {
                    size: storage.size_bytes,
                    alignment: u64::from(storage.alignment_bytes),
                })
            }
        }
    }
}
