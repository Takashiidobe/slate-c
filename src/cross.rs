use anyhow::{Result, anyhow, ensure};
use cranelift_codegen::{
    isa,
    settings::{self, Configurable, SetError, SettingKind},
};
use cranelift_module::default_libcall_names;
use cranelift_object::{ObjectBuilder, ObjectModule};
use serde::{Deserialize, Serialize};
use slate_parser::{
    compiler_args::CompilerArgParser, dialect::Dialect, parser::Parser, sema::Sema,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompileRequest {
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub c_flags: Vec<String>,
    #[serde(default)]
    pub cranelift_flags: Vec<String>,
}

#[derive(Serialize)]
pub struct Compilation {
    pub target: String,
    pub clif: String,
    pub object: Vec<u8>,
    pub functions: Vec<crate::object::FunctionOutput>,
    pub entry: String,
}

#[derive(Serialize)]
pub struct FlagInfo {
    name: String,
    description: String,
    kind: &'static str,
    values: Vec<String>,
    default: Option<String>,
    shared: bool,
}

pub fn flag_schema(target: &str) -> Result<Vec<FlagInfo>> {
    let triple: target_lexicon::Triple = target.parse().map_err(|e| anyhow!("{e}"))?;
    let backend = isa::lookup(triple.clone()).map_err(|e| anyhow!("{e}"))?;
    let shared = settings::builder();
    let defaults = isa::lookup(triple)
        .map_err(|e| anyhow!("{e}"))?
        .finish(settings::Flags::new(shared.clone()))?;
    let values: std::collections::HashMap<_, _> = defaults
        .flags()
        .iter()
        .chain(defaults.isa_flags())
        .map(|flag| (flag.name.to_owned(), flag.value_string()))
        .collect();
    let info = |setting: settings::Setting, shared| FlagInfo {
        name: setting.name.to_owned(),
        description: setting.description.to_owned(),
        kind: match setting.kind {
            SettingKind::Bool => "bool",
            SettingKind::Enum => "enum",
            SettingKind::Num => "num",
            SettingKind::Preset => "preset",
        },
        values: setting
            .values
            .unwrap_or(&[])
            .iter()
            .map(|v| (*v).to_owned())
            .collect(),
        default: values.get(setting.name).cloned(),
        shared,
    };
    Ok(shared
        .iter()
        .map(|setting| info(setting, true))
        .chain(backend.iter().map(|setting| info(setting, false)))
        .collect())
}

pub fn compile(request: &CompileRequest) -> Result<Compilation> {
    ensure!(
        matches!(
            request.target.as_str(),
            "x86_64-unknown-linux-gnu"
                | "aarch64-unknown-linux-gnu"
                | "x86_64-unknown-linux-musl"
                | "aarch64-unknown-linux-musl"
        ),
        "unsupported target {}",
        request.target
    );
    for flag in &request.c_flags {
        ensure!(
            !flag.starts_with("-O"),
            "C optimization flags are not implemented; use Cranelift opt_level"
        );
    }
    let mut arguments = vec![format!("--target={}", request.target)];
    arguments.extend(request.c_flags.iter().cloned());
    let args = CompilerArgParser::parse(arguments)?;
    ensure!(
        args.target.triple == request.target,
        "C flags conflict with the selected target"
    );
    ensure!(
        args.options.codegen == slate_parser::compiler_options::CodegenOptions::default(),
        "C code generation options are not implemented"
    );
    let search = args.search_paths()?;
    let dialect = Dialect::new(args.flavor, args.standard, args.target, args.options);
    let mut parser =
        Parser::new(search, dialect).with_preprocessor_inputs(args.preprocessor_inputs);
    let (unit, files) = parser
        .parse_file_with_source(std::path::Path::new("input.c"), request.source.clone())
        .map_err(|e| anyhow!("{e}"))?;
    let mut sema = Sema::new(&unit);
    sema.names().map_err(|e| anyhow!("{e}"))?;
    sema.analyze(&files).map_err(crate::frontend_errors)?;
    let (slate, _) = sema.lower(&files).map_err(crate::frontend_errors)?;

    let mut shared = settings::builder();
    let triple = request.target.parse().map_err(|e| anyhow!("{e}"))?;
    let mut backend = isa::lookup(triple).map_err(|e| anyhow!("{e}"))?;
    for flag in &request.cranelift_flags {
        let (name, value) = match flag.trim().split_once('=') {
            Some((name, value)) => (name.trim(), Some(value.trim())),
            None => (flag.trim(), None),
        };
        let apply = |builder: &mut dyn Configurable| match value {
            Some(value) => builder.set(name, value),
            None => builder.enable(name),
        };
        match apply(&mut shared) {
            Err(SetError::BadName(_)) => apply(&mut backend)?,
            result => result?,
        }
    }
    let isa = backend.finish(settings::Flags::new(shared))?;
    let mut module =
        ObjectModule::new(ObjectBuilder::new(isa, "slate-c", default_libcall_names())?);
    let output = crate::object::lower(&slate, &mut module)?;
    Ok(Compilation {
        target: request.target.clone(),
        clif: output.clif,
        object: module.finish().emit()?,
        functions: output.functions,
        entry: output.entry,
    })
}
