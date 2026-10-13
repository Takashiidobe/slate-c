use anyhow::{Result, anyhow};
use slate_parser::{
    compiler_args::CompilerFlavor, dialect::Dialect, files::SearchPaths, ir as slate,
    parser::Parser, sema::Sema, target_info::TargetInfo,
};

pub fn parse(source: &str, target: &str) -> Result<slate::Module> {
    let target = TargetInfo::for_triple(target)?;
    let dialect = Dialect::for_flavor(CompilerFlavor::Clang, target);
    let mut parser = Parser::new(SearchPaths::default(), dialect);
    parse_with(&mut parser, source)
}

pub(crate) fn parse_with(parser: &mut Parser, source: &str) -> Result<slate::Module> {
    let (unit, files) = parser
        .parse_file_with_source(std::path::Path::new("input.c"), source.to_owned())
        .map_err(|e| anyhow!("{e}"))?;
    let mut sema = Sema::new(&unit);
    sema.names().map_err(|e| anyhow!("{e}"))?;
    sema.analyze(&files).map_err(frontend_errors)?;
    let (module, _) = sema.lower(&files).map_err(frontend_errors)?;
    Ok(module)
}

pub(crate) fn frontend_errors(errors: slate_parser::sema::SemaErrors) -> anyhow::Error {
    anyhow!(
        "{}",
        errors
            .errors
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    )
}
