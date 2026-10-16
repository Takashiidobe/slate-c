use crate::diagnostics::Result;
use slate_parser::{
    compiler_args::CompilerFlavor, dialect::Dialect, files::SearchPaths, ir as slate,
    parser::Parser, sema::Sema, target_info::TargetInfo,
};

pub fn parse(source: &str, target: &str) -> Result<slate::Module> {
    Ok(parse_with_sources(source, target)?.module)
}

pub(crate) struct Parsed {
    pub module: slate::Module,
    #[cfg(feature = "browser")]
    pub ast_sexpr: String,
    #[cfg(any(feature = "native", feature = "browser"))]
    pub files: slate_parser::files::Files,
}

pub(crate) fn parse_with_sources(source: &str, target: &str) -> Result<Parsed> {
    let target = TargetInfo::for_triple(target)?;
    let dialect = Dialect::for_flavor(CompilerFlavor::Clang, target);
    let mut parser = Parser::new(SearchPaths::default(), dialect);
    parse_with(&mut parser, source)
}

pub(crate) fn parse_with(parser: &mut Parser, source: &str) -> Result<Parsed> {
    let (unit, files) =
        parser.parse_file_with_source(std::path::Path::new("input.c"), source.to_owned())?;
    let mut sema = Sema::new(&unit);
    sema.analyze(&files).map_err(|error| {
        crate::diagnostics::CompilerError::from(error).with_sources(&files, source)
    })?;
    let (module, _) = sema.lower(&files).map_err(|error| {
        crate::diagnostics::CompilerError::from(error).with_sources(&files, source)
    })?;
    let mut ast_sexpr = Vec::new();
    slate_parser::render::SExprRenderer::new(&mut ast_sexpr).render(&unit)?;
    Ok(Parsed {
        module,
        #[cfg(feature = "browser")]
        ast_sexpr: String::from_utf8_lossy(&ast_sexpr).into_owned(),
        #[cfg(any(feature = "native", feature = "browser"))]
        files,
    })
}
