use anyhow::{Result, ensure};
use filecheck::CheckerBuilder;
use std::fs;
use std::path::{Path, PathBuf};

pub fn discover(directory: &Path, fixtures: &mut Vec<PathBuf>) -> Result<()> {
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

pub fn check(source: &str, channel: &str, output: &str, required: bool) -> Result<()> {
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

pub fn values<'a>(source: &'a str, key: &str) -> Vec<&'a str> {
    source
        .lines()
        .filter_map(|line| line.trim().strip_prefix("//").map(str::trim))
        .filter_map(|line| line.strip_prefix(key).map(str::trim))
        .collect()
}

pub fn one<'a>(source: &'a str, key: &str) -> Result<Option<&'a str>> {
    let values = values(source, key);
    ensure!(values.len() <= 1, "duplicate {key} directive");
    Ok(values.first().copied())
}
