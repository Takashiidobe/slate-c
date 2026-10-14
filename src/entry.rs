use crate::diagnostics::{CompilerError, Result};
use slate_parser::ir::{self as c, NumericType, Type};

pub(crate) fn main_binding(slate: &c::Module) -> Result<c::BindingId> {
    let main = slate
        .functions
        .iter()
        .find(|f| f.value.name == "main" && f.value.body.is_some())
        .ok_or_else(|| CompilerError::Configuration("missing main".into()))?;
    if !(matches!(&main.value.parameters, c::Parameters::Prototype { fixed, variadic: false } if fixed.is_empty())
        && matches!(
            main.value.return_type.as_ref(),
            Some(Type::Numeric(NumericType::Integer {
                width: 32,
                signed: true,
                ..
            }))
        ))
    {
        return Err(
            CompilerError::Unsupported("execution requires int main(void)".to_string())
                .at(main.expansion),
        );
    };
    Ok(main.value.id)
}
