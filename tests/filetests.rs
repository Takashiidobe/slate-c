use anyhow::{Context, Result, bail, ensure};
use filecheck::CheckerBuilder;
use libtest_mimic::{Arguments, Trial};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use wait_timeout::ChildExt;

fn discover(directory: &Path, fixtures: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            discover(&path, fixtures)?;
        } else if path.extension().is_some_and(|extension| extension == "c") {
            fixtures.push(path);
        }
    }
    Ok(())
}

fn check(source: &str, channel: &str, output: &str, required: bool) -> Result<()> {
    let mut builder = CheckerBuilder::new();
    builder.directive(r"regex: var=v[0-9]+")?;
    builder.directive(r"regex: block=block[0-9]+")?;
    builder.directive(r"regex: func=fn[0-9]+")?;
    let mut checks = 0;
    let mut empty = false;
    for line in source.lines() {
        let Some(line) = line.trim().strip_prefix("//").map(str::trim) else {
            continue;
        };
        let Some(directive) = line.strip_prefix(&format!("{channel}:")) else {
            continue;
        };
        let directive = directive.trim();
        if directive == "empty" {
            empty = true;
        } else {
            ensure!(
                builder.directive(directive)?,
                "unknown {channel} directive: {directive}"
            );
            if !directive.starts_with("regex:") {
                checks += 1;
            }
        }
    }
    ensure!(
        !(empty && checks > 0),
        "{channel}: empty cannot be combined with checks"
    );
    if empty || (checks == 0 && !required) {
        ensure!(
            output.is_empty(),
            "expected empty {channel}, got:\n{output}"
        );
    } else {
        ensure!(checks > 0, "missing {channel} checks");
        let (matched, explanation) = builder.finish().explain(output, &())?;
        ensure!(
            matched,
            "{channel} mismatch:\n{explanation}\nactual:\n{output}"
        );
    }
    Ok(())
}

fn run(path: &Path) -> Result<()> {
    let source = fs::read_to_string(path)?;
    let directives: Vec<_> = source
        .lines()
        .filter_map(|line| line.trim().strip_prefix("//").map(str::trim))
        .collect();
    let modes: Vec<_> = directives
        .iter()
        .filter_map(|line| line.strip_prefix("MODE:"))
        .collect();
    ensure!(modes.len() <= 1, "duplicate MODE directive");
    let mode = modes.first().map_or("run", |mode| mode.trim());
    ensure!(
        matches!(mode, "run" | "compile" | "error"),
        "unknown MODE: {mode}"
    );
    let compilation = slate_c::native::compile(&source);
    if mode == "error" {
        let error = match compilation {
            Ok(_) => bail!("expected compilation to fail"),
            Err(error) => slate_c::diagnostics::render(&error),
        };
        return check(&source, "ERROR", &error, true);
    }
    let compilation = compilation?;
    if directives.iter().any(|line| line.starts_with("CLIF:")) {
        check(&source, "CLIF", &compilation.clif, true)?;
    } else {
        ensure!(mode != "compile", "compile fixtures need CLIF checks");
    }
    if mode == "compile" {
        return Ok(());
    }
    ensure!(
        cfg!(all(target_arch = "x86_64", target_os = "linux")),
        "native execution fixtures require x86_64 Linux"
    );
    let exits: Vec<_> = directives
        .iter()
        .filter_map(|line| line.strip_prefix("EXIT:"))
        .collect();
    ensure!(
        exits.len() == 1,
        "run fixtures need exactly one EXIT directive"
    );
    let expected: i32 = exits[0].trim().parse().context("EXIT must be an integer")?;
    ensure!((0..=255).contains(&expected), "EXIT must be in 0..=255");
    let directory = tempfile::tempdir()?;
    let object = directory.path().join("input.o");
    let executable = directory.path().join("program");
    fs::write(&object, compilation.object)?;
    let linker = Command::new(std::env::var_os("CC").unwrap_or_else(|| "cc".into()))
        .arg(&object)
        .arg("-o")
        .arg(&executable)
        .arg("-lm")
        .output()
        .context("invoke native C linker (CC, default cc)")?;
    ensure!(
        linker.status.success(),
        "link failed:\n{}",
        String::from_utf8_lossy(&linker.stderr)
    );
    let stdout = directory.path().join("stdout");
    let stderr = directory.path().join("stderr");
    let mut child = Command::new(&executable)
        .current_dir(directory.path())
        .stdin(Stdio::null())
        .stdout(File::create(&stdout)?)
        .stderr(File::create(&stderr)?)
        .spawn()?;
    let Some(status) = child.wait_timeout(Duration::from_secs(5))? else {
        child.kill()?;
        child.wait()?;
        bail!("program timed out after five seconds");
    };
    let stdout = fs::read_to_string(stdout)?;
    let stderr = fs::read_to_string(stderr)?;
    let code = status.code().with_context(|| {
        format!("program terminated with {status}; stdout:\n{stdout}\nstderr:\n{stderr}")
    })?;
    check(
        &format!(r"// EXIT: check: $(=\A{expected}\n\z)"),
        "EXIT",
        &format!("{code}\n"),
        true,
    )
    .with_context(|| {
        format!("expected exit {expected}, got {status}; stdout:\n{stdout}\nstderr:\n{stderr}")
    })?;
    check(&source, "STDOUT", &stdout, false)?;
    check(&source, "STDERR", &stderr, false)
}

fn main() {
    let arguments = Arguments::from_args();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/filetests");
    let mut paths = Vec::new();
    discover(&root, &mut paths).expect("discover filetest fixtures");
    paths.sort();
    assert!(!paths.is_empty(), "no filetest fixtures found");
    let tests = paths
        .into_iter()
        .map(|path| {
            let name = path
                .strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            Trial::test(name, move || {
                run(&path).map_err(|error| format!("{error:#}").into())
            })
        })
        .collect();
    libtest_mimic::run(&arguments, tests).exit();
}
