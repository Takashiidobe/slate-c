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
