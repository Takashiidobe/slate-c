mod support;

use anyhow::{Context, Result, bail, ensure};
use libtest_mimic::{Arguments, Trial};
use slate_c::cross::{CompileRequest, compile};
use std::fs;
use std::path::Path;
use support::{check, discover, one, values};

fn run(path: &Path, target: &str) -> Result<()> {
    let source = fs::read_to_string(path)?;
    let request = CompileRequest {
        source: source.clone(),
        target: target.into(),
        c_flags: values(&source, "C-FLAGS:")
            .iter()
            .flat_map(|line| line.split_whitespace().map(str::to_owned))
            .collect(),
        cranelift_flags: values(&source, "CLIF-FLAGS:")
            .iter()
            .flat_map(|line| line.split_whitespace().map(str::to_owned))
            .collect(),
    };
    let output = compile(&request);
    if !values(&source, "ERROR:").is_empty() {
        let error = output.err().context("expected cross compilation to fail")?;
        return check(
            &source,
            "ERROR",
            &slate_c::diagnostics::render(&error),
            true,
        );
    }
    let output = output?;
    ensure!(output.object.starts_with(b"\x7fELF"), "expected ELF object");
    let machine = match target.split('-').next() {
        Some("x86_64") => 62,
        Some("aarch64") => 183,
        _ => bail!("unsupported fixture target {target}"),
    };
    ensure!(
        u16::from_le_bytes(output.object[18..20].try_into()?) == machine,
        "incorrect ELF machine"
    );
    if let Some(count) = one(&source, "FUNCTIONS:")? {
        ensure!(
            output.functions.len() == count.parse::<usize>()?,
            "incorrect function count"
        );
    }
    if !values(&source, "CLIF:").is_empty() {
        check(&source, "CLIF", &output.clif, true)?;
    }
    Ok(())
}

fn main() {
    let arguments = Arguments::from_args();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/filetests/cross");
    let mut paths = Vec::new();
    discover(&root, &mut paths).expect("discover cross fixtures");
    paths.sort();
    assert!(!paths.is_empty(), "no cross fixtures found");
    let mut tests = Vec::new();
    for path in paths {
        let source = fs::read_to_string(&path).expect("read cross fixture");
        let targets = values(&source, "TARGET:");
        assert!(
            !targets.is_empty(),
            "{} needs TARGET directives",
            path.display()
        );
        for target in targets {
            let name = format!("{}/{}", path.strip_prefix(&root).unwrap().display(), target);
            let path = path.clone();
            let target = target.to_owned();
            tests.push(Trial::test(name, move || {
                run(&path, &target).map_err(|error| format!("{error:#}").into())
            }));
        }
    }
    libtest_mimic::run(&arguments, tests).exit();
}
