#[cfg(feature = "native")]
const RETURN_42: &str = include_str!("fixtures/return42.c");

#[test]
#[cfg(feature = "native")]
fn jit_resolves_f80_runtime_helpers() {
    assert_eq!(
        slate_c::native::run(include_str!("filetests/runtime/f80_precision.c")).unwrap(),
        42
    );
}

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
fn execution_requires_a_defined_scalar_entry_point() {
    for source in [
        "int main(void) { return ;",
        "int main(int argc) { return 42; }",
        "int other(void) { return 42; }",
        "int main(void);",
    ] {
        assert!(slate_c::native::run(source).is_err(), "accepted {source}");
    }
}

#[test]
#[cfg(feature = "native")]
fn compiles_translation_units_without_an_entry_point() {
    let object =
        slate_c::native::compile_object("int global = 7; int other(int x) { return global + x; }")
            .unwrap();
    assert!(!object.is_empty());
    assert!(slate_c::native::compile_object("int main(void);").is_ok());
}

#[test]
#[cfg(feature = "native")]
fn executes_shared_function_and_memory_lowering() {
    let source = "
        int global = 2;
        int add(int x, int y) { return x + y; }
        int main(void) {
            int sum = 0;
            for (int i = 0; i < 5; i++) {
                if (i == 3) continue;
                sum = add(sum, i);
            }
            int values[2]; values[0] = sum; values[1] = global;
            return values[0] * values[1];
        }
    ";
    assert_eq!(slate_c::native::run(source).unwrap(), 14);
    assert!(!slate_c::native::compile_object(source).unwrap().is_empty());
}

#[test]
#[cfg(feature = "native")]
fn resolves_types_and_record_layouts_in_each_module() {
    for (element, expected) in [("int", 11), ("long long", 19)] {
        let source = format!(
            "typedef {element} Number;
             struct Pair {{ Number x; Number y; }};
             struct Pair global;
             int main(void) {{
                 struct Pair values[2];
                 values[1].x = 1;
                 global.y = 2;
                 return values[1].x + global.y + sizeof(struct Pair);
             }}"
        );
        assert_eq!(slate_c::native::run(&source).unwrap(), expected);
    }
}
