mod support;

use anyhow::{Context, Result, bail, ensure};
use libtest_mimic::{Arguments, Trial};
use miette::Diagnostic;
use std::fs::{self, File};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;
use support::{check, discover, one, values};
use wait_timeout::ChildExt;

fn run(path: &Path, jit: bool) -> Result<()> {
    let source = fs::read_to_string(path)?;
    let mode = one(&source, "MODE:")?.unwrap_or("run");
    ensure!(
        matches!(mode, "run" | "compile" | "error" | "jit-error"),
        "unknown MODE: {mode}"
    );
    if mode == "error" || mode == "jit-error" {
        let error = if mode == "jit-error" {
            slate_c::native::run(&source).err()
        } else {
            slate_c::native::compile(&source).err()
        }
        .context("expected compilation to fail")?;
        check(
            &source,
            "ERROR",
            &slate_c::diagnostics::render(&error),
            true,
        )?;
        diagnostic(&source, &error)?;
        let directory = tempfile::tempdir()?;
        let object = directory.path().join("output.o");
        let mut command = Command::new(env!("CARGO_BIN_EXE_slate-c"));
        if mode == "jit-error" {
            command.arg("run").arg(path);
        } else {
            command.arg("object").arg(path).arg(&object);
        }
        let output = command.output()?;
        ensure!(
            output.status.code() == Some(1),
            "unexpected CLI status: {}",
            output.status
        );
        ensure!(output.stdout.is_empty(), "diagnostic CLI wrote stdout");
        let stderr = String::from_utf8(output.stderr)?;
        check(
            &source,
            "ERROR",
            &stderr.replace(path.to_str().context("non-UTF-8 fixture path")?, "input.c"),
            true,
        )?;
        if error.source_code().is_some() {
            ensure!(
                stderr.contains(&path.to_string_lossy().to_string()),
                "CLI lost input path"
            );
        }
        ensure!(!object.exists(), "failed compilation emitted an object");
        return Ok(());
    }
    let compilation = slate_c::native::compile(&source)?;
    ensure!(
        compilation.object.starts_with(b"\x7fELF"),
        "expected ELF object"
    );
    if !values(&source, "CLIF:").is_empty() {
        check(&source, "CLIF", &compilation.clif, true)?;
    }
    if mode == "compile" {
        return Ok(());
    }
    ensure!(
        cfg!(all(
            any(target_arch = "x86_64", target_arch = "aarch64"),
            target_os = "linux"
        )),
        "native execution fixtures require x86_64 or AArch64 Linux"
    );
    let expected: i32 = one(&source, "EXIT:")?
        .context("run fixtures need an EXIT directive")?
        .parse()
        .context("EXIT must be an integer")?;
    ensure!((0..=255).contains(&expected), "EXIT must be in 0..=255");
    let directory = tempfile::tempdir()?;
    let object = directory.path().join("input.o");
    let executable = directory.path().join("program");
    if !jit {
        fs::write(&object, compilation.object)?;
        let linker = Command::new(std::env::var_os("CC").unwrap_or_else(|| "cc".into()))
            .arg(&object)
            .arg(slate_c::runtime::ARCHIVE)
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
    }
    let stdout = directory.path().join("stdout");
    let stderr = directory.path().join("stderr");
    let mut command = if jit {
        let mut command = Command::new(std::env::current_exe()?);
        command.arg("--jit-fixture").arg(path);
        command
    } else {
        Command::new(&executable)
    };
    let mut child = command
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

fn diagnostic(source: &str, error: &slate_c::diagnostics::CompilerError) -> Result<()> {
    use slate_c::diagnostics::CompilerError;

    if let Some(kind) = one(source, "DIAGNOSTIC:")? {
        let matches = match kind {
            "lowering" => {
                matches!(error, CompilerError::Located(located) if matches!(located.error, CompilerError::Unsupported(_)))
            }
            "frontend" => matches!(error, CompilerError::Frontend(_)),
            "semantic" => matches!(error, CompilerError::Semantic(_)),
            _ => bail!("unknown diagnostic kind {kind}"),
        };
        ensure!(matches, "unexpected diagnostic kind: {error:?}");
        if kind != "semantic" {
            ensure!(error.source_code().is_some(), "diagnostic lost its source");
        }
    }
    if let Some(marker) = one(source, "LABEL:")? {
        ensure!(error.source_code().is_some(), "label lost its source");
        let offset = source
            .rfind(marker)
            .context("label marker absent from source")?;
        let labels: Vec<_> = error.labels().into_iter().flatten().collect();
        ensure!(labels.len() == 1, "expected one primary label");
        ensure!(
            labels[0].offset() == offset,
            "incorrect label offset: {} != {offset}",
            labels[0].offset()
        );
        let minimum = one(source, "LABEL-MIN:")?
            .map(str::parse::<usize>)
            .transpose()?
            .unwrap_or(1);
        ensure!(labels[0].len() >= minimum, "label is too short");
        #[cfg(feature = "browser")]
        {
            let payload = slate_c::diagnostics::to_json(error);
            ensure!(
                payload["labels"][0]["offset"] == offset,
                "JSON lost label offset"
            );
            ensure!(
                payload["labels"][0]["file"] == "input.c",
                "JSON lost source file"
            );
            ensure!(
                payload["labels"][0]["line"]
                    == source[..offset]
                        .bytes()
                        .filter(|byte| *byte == b'\n')
                        .count()
                        + 1,
                "JSON lost source line"
            );
            ensure!(payload["severity"] == "error", "JSON lost severity");
            ensure!(
                payload["rendered"] == slate_c::diagnostics::render(error),
                "JSON lost rendered diagnostic"
            );
        }
    }
    if let Some(count) = one(source, "RELATED:")? {
        let related: Vec<_> = error.related().into_iter().flatten().collect();
        ensure!(
            related.len() >= count.parse::<usize>()?,
            "missing related diagnostics"
        );
        for diagnostic in &related {
            ensure!(
                diagnostic.source_code().is_some(),
                "related error lost source"
            );
            ensure!(
                diagnostic.labels().into_iter().flatten().next().is_some(),
                "related error lost labels"
            );
        }
        #[cfg(feature = "browser")]
        {
            let payload = slate_c::diagnostics::to_json(error);
            ensure!(
                payload["related"].as_array().unwrap().len() == related.len(),
                "JSON lost related errors"
            );
            for related in payload["related"].as_array().unwrap() {
                ensure!(
                    !related["labels"].as_array().unwrap().is_empty(),
                    "JSON lost related labels"
                );
            }
        }
    }
    if let Some(code) = one(source, "CODE:")? {
        ensure!(
            error.code().map(|value| value.to_string()).as_deref() == Some(code),
            "incorrect diagnostic code"
        );
        #[cfg(feature = "browser")]
        ensure!(
            slate_c::diagnostics::to_json(error)["code"] == code,
            "JSON lost diagnostic code"
        );
    }
    Ok(())
}

fn main() {
    let mut worker_args = std::env::args_os().skip(1);
    if worker_args.next().as_deref() == Some(std::ffi::OsStr::new("--jit-fixture")) {
        let path = worker_args.next().expect("JIT fixture path");
        let source = fs::read_to_string(path).expect("read JIT fixture");
        match slate_c::native::run(&source) {
            Ok(code) => std::process::exit(code),
            Err(error) => {
                eprintln!("{}", slate_c::diagnostics::render(&error));
                std::process::exit(1);
            }
        }
    }
    let arguments = Arguments::from_args();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/filetests");
    let mut paths = Vec::new();
    discover(&root, &mut paths).expect("discover filetest fixtures");
    paths.retain(|path| !path.starts_with(root.join("cross")));
    paths.sort();
    assert!(!paths.is_empty(), "no filetest fixtures found");
    let tests = paths
        .into_iter()
        .flat_map(|path| {
            let name = path
                .strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            let mut tests = Vec::new();
            if path.starts_with(root.join("runtime")) {
                let jit_path = path.clone();
                tests.push(Trial::test(format!("jit/{name}"), move || {
                    run(&jit_path, true).map_err(|error| format!("{error:#}").into())
                }));
            }
            tests.push(Trial::test(name, move || {
                run(&path, false).map_err(|error| format!("{error:#}").into())
            }));
            tests
        })
        .collect();
    libtest_mimic::run(&arguments, tests).exit();
}
