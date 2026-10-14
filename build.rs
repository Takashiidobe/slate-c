use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=runtime/softfloat");
    if env::var_os("CARGO_FEATURE_NATIVE").is_none() {
        return;
    }
    assert_eq!(env::var("CARGO_CFG_TARGET_ARCH").unwrap(), "x86_64");
    assert_eq!(env::var("CARGO_CFG_TARGET_OS").unwrap(), "linux");
    let root = PathBuf::from("runtime/softfloat");
    let mut build = cc::Build::new();
    build
        .std("c11")
        .opt_level(2)
        .warnings(false)
        .include(&root)
        .include(root.join("vendor/source/include"))
        .include(root.join("vendor/source/8086-SSE"));
    for definition in fs::read_to_string(root.join("defines.txt"))
        .unwrap()
        .lines()
    {
        let (name, value) = definition
            .split_once('=')
            .map_or((definition, None), |(name, value)| (name, Some(value)));
        build.define(name, value);
    }
    for source in fs::read_to_string(root.join("sources.txt"))
        .unwrap()
        .lines()
    {
        build.file(root.join(source));
    }
    build.file(root.join("helpers.c"));
    build.compile("slate_f80");
    let archive = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("libslate_f80.a");
    fs::copy(
        root.join("vendor/COPYING.txt"),
        archive.with_file_name("SOFTFLOAT-LICENSE.txt"),
    )
    .unwrap();
    println!("cargo:rustc-env=SLATE_F80_ARCHIVE={}", archive.display());
}
