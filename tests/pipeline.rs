#[cfg(feature = "native")]
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
#[cfg(feature = "native")]
fn rejects_invalid_and_unsupported_programs() {
    for source in [
        "int main(void) { return ;",
        "int main(int argc) { return 42; }",
        "int main(void) { int x = 42; return x; }",
        "int other(void) { return 42; }",
        "int main(void);",
        "int global; int main(void) { return 42; }",
    ] {
        assert!(
            slate_c::native::compile_object(source).is_err(),
            "accepted {source}"
        );
    }
}
