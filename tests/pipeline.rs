use std::process::Command;

const RETURN_42: &str = include_str!("fixtures/return42.c");

#[cfg(feature = "native")]
#[test]
fn slate_to_native_returns_42() {
    assert_eq!(slate_c::native::run(RETURN_42).unwrap(), 42);
    assert_eq!(
        slate_c::native::run("int main(void) { return 17; }").unwrap(),
        17
    );
}

#[test]
fn slate_to_wasm_returns_the_source_constant() {
    for (source, expected) in [(RETURN_42, 42), ("int main(void) { return 17; }", 17)] {
        let wasm = slate_c::compile_wasm(source).unwrap();
        let mut child = Command::new("node")
            .args(["-e", "const chunks=[]; process.stdin.on('data', x => chunks.push(x)); process.stdin.on('end', async () => { const bytes=Buffer.concat(chunks); if (!WebAssembly.validate(bytes)) process.exit(1); const {instance}=await WebAssembly.instantiate(bytes); console.log(instance.exports.main()); });"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("Node.js is required to execute the emitted WebAssembly");
        use std::io::Write;
        child.stdin.take().unwrap().write_all(&wasm).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim(),
            expected.to_string()
        );
    }
}

#[test]
fn rejects_invalid_and_unsupported_programs() {
    for source in [
        "int main(void) { return ;",
        "int main(int argc) { return 42; }",
        "int main(void) { int x = 42; return x; }",
        "int other(void) { return 42; }",
        "int main(void);",
        "int global; int main(void) { return 42; }",
    ] {
        assert!(slate_c::compile_wasm(source).is_err(), "accepted {source}");
    }
}
