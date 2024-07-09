use std::env;
use std::path::PathBuf;

const ABC_LIB_NAMES_LINES: &str = include_str!("abc_lib_names.txt");

fn abc_lib_names() -> impl Iterator<Item = &'static str> {
    ABC_LIB_NAMES_LINES.split_terminator('\n')
}

fn main() {
    println!("cargo:rerun-if-env-changed=VTS_ABC_BUILD_DIR");
    println!("cargo:rerun-if-changed=abc_lib_names.txt");
    println!("cargo:rerun-if-changed=CMakeLists.txt");
    let abc_srcdir = PathBuf::from("abc/src");
    if abc_srcdir.is_dir() {
        println!("cargo:rerun-if-changed={}", abc_srcdir.display());
    }
    let vts_abc_sys_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let abc_build_dir = match env::var_os("VTS_ABC_BUILD_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => cmake::build(&vts_abc_sys_dir),
    };
    cc::Build::new()
        .cpp(true)
        .file("wrapper.cpp")
        .compile("vts-abc-wrapper");
    println!(
        "cargo:rustc-link-search=native={}/lib",
        abc_build_dir.display()
    );
    for lib_name in abc_lib_names() {
        println!("cargo:rustc-link-lib=static:+whole-archive={lib_name}");
    }
}
