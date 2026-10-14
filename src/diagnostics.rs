use miette::{Diagnostic, NamedSource, SourceSpan};
use slate_parser::{ast::Loc, files::Files};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, CompilerError>;

#[derive(Debug, Error, Diagnostic)]
pub enum CompilerError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Frontend(Box<slate_parser::error::FrontendError>),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Semantic(Box<slate_parser::sema::SemaErrors>),
    #[error("{0}")]
    #[diagnostic(code(slate_c::unsupported))]
    Unsupported(String),
    #[error("invalid compiler IR: {0}")]
    #[diagnostic(code(slate_c::invalid_ir))]
    InvalidIr(String),
    #[error("{0}")]
    #[diagnostic(code(slate_c::configuration))]
    Configuration(String),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Located(Box<SourceError>),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Integer(#[from] std::num::ParseIntError),
    #[error(transparent)]
    Size(#[from] std::num::TryFromIntError),
    #[error(transparent)]
    Target(#[from] slate_parser::target_info::TargetError),
    #[error("{0}")]
    TargetTriple(target_lexicon::ParseError),
    #[error(transparent)]
    Setting(#[from] cranelift_codegen::settings::SetError),
    #[error(transparent)]
    Codegen(#[from] cranelift_codegen::CodegenError),
    #[error(transparent)]
    Isa(#[from] cranelift_codegen::isa::LookupError),
    #[cfg(any(feature = "native", feature = "browser"))]
    #[error(transparent)]
    Module(Box<cranelift_module::ModuleError>),
    #[cfg(any(feature = "native", feature = "browser"))]
    #[error(transparent)]
    Object(#[from] cranelift_object::object::write::Error),
    #[cfg(feature = "browser")]
    #[error(transparent)]
    Arguments(#[from] slate_parser::compiler_args::CompilerArgError),
    #[cfg(feature = "browser")]
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Error, Diagnostic)]
#[error("{error}")]
#[diagnostic(code(slate_c::lowering))]
pub struct SourceError {
    #[source]
    pub error: CompilerError,
    pub location: Loc,
    #[source_code]
    pub source: Option<NamedSource<String>>,
    #[label("lowering failed here")]
    pub span: Option<SourceSpan>,
}

impl CompilerError {
    #[cfg(any(feature = "native", feature = "browser"))]
    pub(crate) fn at(self, location: Loc) -> Self {
        if matches!(self, Self::Located(_)) {
            return self;
        }
        Self::Located(Box::new(SourceError {
            error: self,
            location,
            source: None,
            span: None,
        }))
    }

    pub fn with_input_name(mut self, name: &str) -> Self {
        fn rename(source: &mut NamedSource<String>, name: &str) {
            if source.name() == "input.c" {
                *source = NamedSource::new(name, source.inner().clone()).with_language("C");
            }
        }
        fn rename_semantic(errors: &mut slate_parser::sema::SemaErrors, name: &str) {
            for error in &mut errors.errors {
                rename(&mut error.source_code, name);
            }
        }
        match &mut self {
            Self::Frontend(error) => match error.as_mut() {
                slate_parser::error::FrontendError::PP(error) => {
                    rename(&mut error.source_code, name)
                }
                slate_parser::error::FrontendError::Parse(error) => {
                    rename(&mut error.source_code, name)
                }
                slate_parser::error::FrontendError::Sema(errors) => rename_semantic(errors, name),
            },
            Self::Semantic(errors) => rename_semantic(errors, name),
            Self::Located(error) => {
                if let Some(source) = &mut error.source {
                    rename(source, name);
                }
            }
            _ => {}
        }
        self
    }

    pub(crate) fn with_sources(mut self, files: &Files, input: &str) -> Self {
        if let Self::Semantic(errors) = &mut self {
            for error in &mut errors.errors {
                if let Some(location) = error.loc
                    && files.get_path(location.file) == Some(std::path::Path::new("input.c"))
                {
                    let offset = location.offset.min(input.len());
                    let length = location.length.min(input.len() - offset);
                    error.source_code =
                        NamedSource::new("input.c", input.to_owned()).with_language("C");
                    error.span = (offset, length).into();
                }
            }
        }
        if let Self::Located(error) = &mut self
            && let Some(path) = files.get_path(error.location.file)
        {
            let source = if path == std::path::Path::new("input.c") {
                Some(input.to_owned())
            } else {
                std::fs::read_to_string(path).ok()
            };
            if let Some(source) = source {
                let offset = error.location.offset;
                let end = offset.checked_add(error.location.length);
                if end.is_some_and(|end| end <= source.len()) {
                    error.span = Some((offset, error.location.length).into());
                    error.source =
                        Some(NamedSource::new(path.to_string_lossy(), source).with_language("C"));
                }
            }
        }
        self
    }
}

impl From<slate_parser::error::FrontendError> for CompilerError {
    fn from(error: slate_parser::error::FrontendError) -> Self {
        Self::Frontend(Box::new(error))
    }
}

impl From<target_lexicon::ParseError> for CompilerError {
    fn from(error: target_lexicon::ParseError) -> Self {
        Self::TargetTriple(error)
    }
}

impl From<slate_parser::sema::SemaErrors> for CompilerError {
    fn from(error: slate_parser::sema::SemaErrors) -> Self {
        Self::Semantic(Box::new(error))
    }
}

#[cfg(any(feature = "native", feature = "browser"))]
impl From<cranelift_module::ModuleError> for CompilerError {
    fn from(error: cranelift_module::ModuleError) -> Self {
        Self::Module(Box::new(error))
    }
}

pub fn render(diagnostic: &dyn Diagnostic) -> String {
    let mut output = String::new();
    if miette::NarratableReportHandler::new()
        .render_report(&mut output, diagnostic)
        .is_err()
    {
        output = format!("{diagnostic}\n");
        for related in diagnostic.related().into_iter().flatten() {
            output.push_str(&render(related));
        }
    }
    output
}

#[cfg(feature = "browser")]
pub fn to_json(diagnostic: &dyn Diagnostic) -> serde_json::Value {
    let labels: Vec<_> = diagnostic
        .labels()
        .into_iter()
        .flatten()
        .map(|label| {
            let source = diagnostic
                .source_code()
                .and_then(|source| source.read_span(label.inner(), 0, 0).ok());
            serde_json::json!({
                "offset": label.offset(),
                "length": label.len(),
                "label": label.label(),
                "file": source.as_ref().and_then(|source| source.name()),
                "line": source.as_ref().map(|source| source.line() + 1),
                "column": source.as_ref().map(|source| source.column() + 1),
            })
        })
        .collect();
    let related: Vec<_> = diagnostic
        .related()
        .into_iter()
        .flatten()
        .map(to_json)
        .collect();
    serde_json::json!({
        "message": diagnostic.to_string(),
        "code": diagnostic.code().map(|code| code.to_string()),
        "severity": format!("{:?}", diagnostic.severity().unwrap_or(miette::Severity::Error)).to_lowercase(),
        "labels": labels,
        "related": related,
        "rendered": render(diagnostic),
    })
}
