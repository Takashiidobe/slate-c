#![cfg(feature = "native")]

use miette::Diagnostic;
use slate_c::diagnostics::{CompilerError, render};

fn failure(source: &str) -> CompilerError {
    match slate_c::native::compile(source) {
        Ok(_) => panic!("expected compilation failure"),
        Err(error) => error,
    }
}

#[test]
fn lowering_preserves_the_innermost_source_span() {
    let source = "/* λ */\nint main(void) { if (1) { switch (1) { default: return 0; } } }";
    let error = failure(source);
    let labels: Vec<_> = error.labels().unwrap().collect();
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0].offset(), source.find("switch").unwrap());
    assert!(labels[0].len() > "switch".len());
    assert!(error.source_code().is_some());
    let report = render(&error);
    assert!(report.contains("input.c"));
    assert!(report.contains("label at line 2"));
    assert!(report.contains("switch statements are not supported"));
    let CompilerError::Located(located) = error else {
        panic!("expected located lowering error");
    };
    assert!(matches!(located.error, CompilerError::Unsupported(_)));
}

#[test]
fn macro_errors_label_the_expansion_site() {
    let source = "#define SELECT switch\nint main(void) { SELECT (1) { default: return 0; } }";
    let error = failure(source);
    let label = error.labels().unwrap().next().unwrap();
    assert_eq!(label.offset(), source.rfind("SELECT").unwrap());
    assert!(render(&error).contains("label at line 2"));
}

#[test]
fn frontend_parse_errors_keep_source_labels() {
    let error = failure("int main(void) { return ;");
    assert!(matches!(error, CompilerError::Frontend(_)));
    assert!(error.source_code().is_some());
    assert!(error.labels().unwrap().next().is_some());
    assert!(render(&error).contains("input.c"));
}

#[test]
fn semantic_errors_keep_related_diagnostics() {
    let error = failure("int main(void) { return missing + other; }");
    assert!(matches!(error, CompilerError::Semantic(_)));
    let related: Vec<_> = error.related().unwrap().collect();
    assert!(related.len() >= 2);
    for diagnostic in related {
        assert!(diagnostic.source_code().is_some());
        assert!(diagnostic.labels().unwrap().next().is_some());
    }
    let report = render(&error);
    assert!(report.contains("missing"));
    assert!(report.contains("other"));
}

#[test]
fn global_initializers_keep_declaration_locations() {
    let source = "struct Pair { int x; int y; };\nstruct Pair global = { 1, 2 };";
    let error = failure(source);
    assert!(error.source_code().is_some());
    let label = error.labels().unwrap().next().unwrap();
    assert_eq!(label.offset(), source.find("global").unwrap());
}

#[test]
fn native_driver_renders_diagnostics_to_stderr() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("input.c");
    std::fs::write(
        &source,
        "int main(void) { switch (1) { default: return 0; } }",
    )
    .unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_slate-c"))
        .arg("object")
        .arg(&source)
        .arg(directory.path().join("output.o"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("switch statements are not supported"));
    assert!(stderr.contains("label at line 1"));
    assert!(stderr.contains(source.to_str().unwrap()));
    assert!(!directory.path().join("output.o").exists());
}

#[cfg(feature = "browser")]
#[test]
fn browser_payload_preserves_labels_and_related_errors() {
    let source = "int main(void) { switch (1) { default: return 0; } }";
    let error = failure(source);
    let payload = slate_c::diagnostics::to_json(&error);
    assert_eq!(payload["code"], "slate_c::lowering");
    assert_eq!(
        payload["labels"][0]["offset"],
        source.find("switch").unwrap()
    );
    assert_eq!(payload["labels"][0]["file"], "input.c");
    assert_eq!(payload["labels"][0]["line"], 1);
    assert_eq!(payload["severity"], "error");
    assert!(
        payload["rendered"]
            .as_str()
            .unwrap()
            .contains("lowering failed here")
    );
    let semantic = slate_c::diagnostics::to_json(&failure("int main(void) { return missing; }"));
    assert!(!semantic["related"].as_array().unwrap().is_empty());
    assert!(
        !semantic["related"][0]["labels"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
