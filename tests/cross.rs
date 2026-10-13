#![cfg(feature = "browser")]

use slate_c::cross::{CompileRequest, compile};

fn request(target: &str) -> CompileRequest {
    CompileRequest {
        source: "int main(void) { return VALUE; }".into(),
        target: target.into(),
        c_flags: vec!["-DVALUE=42".into(), "-std=c17".into(), "-nostdinc".into()],
        cranelift_flags: vec!["opt_level=speed".into()],
    }
}

#[test]
fn emits_target_selected_elf_with_frontend_defines() {
    for (target, machine) in [
        ("x86_64-unknown-linux-gnu", 62),
        ("x86_64-unknown-linux-musl", 62),
    ] {
        let output = compile(&request(target)).unwrap();
        assert_eq!(&output.object[..4], b"\x7fELF");
        assert_eq!(
            u16::from_le_bytes(output.object[18..20].try_into().unwrap()),
            machine
        );
        assert!(output.clif.contains("iconst.i32 42"), "{}", output.clif);
    }
}

#[test]
fn rejects_conflicting_and_unimplemented_flags() {
    for flag in [
        "-m32",
        "--target=aarch64-unknown-linux-gnu",
        "-O2",
        "-fstack-protector",
    ] {
        let mut input = request("x86_64-unknown-linux-gnu");
        input.c_flags.push(flag.into());
        assert!(compile(&input).is_err(), "accepted {flag}");
    }
    let mut input = request("x86_64-unknown-linux-gnu");
    input.cranelift_flags.push("not_a_setting=true".into());
    assert!(compile(&input).is_err());
    input = request("wasm32-unknown-unknown");
    assert!(compile(&input).is_err());
}

#[test]
fn compiles_library_and_non_runner_main_signatures() {
    for source in [
        "int global = 7; int other(int x) { return global + x; }",
        "int main(int argc, char **argv) { return argc; }",
    ] {
        let mut input = request("x86_64-unknown-linux-gnu");
        input.source = source.into();
        let output = compile(&input).unwrap();
        assert_eq!(&output.object[..4], b"\x7fELF");
        assert_eq!(output.functions.len(), 1);
    }
}

#[test]
fn compiles_calls_and_control_flow_through_the_shared_backend() {
    let mut input = request("x86_64-unknown-linux-gnu");
    input.source = "int add(int a, int b) { return a + b; }
        int main(void) { int sum = 0;
            for (int i = 0; i < 5; i++) sum = add(sum, i);
            return sum; }"
        .into();
    let output = compile(&input).unwrap();
    assert_eq!(output.functions.len(), 2);
    assert!(output.clif.contains("call"));
}
