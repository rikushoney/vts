use std::env;
use std::path::PathBuf;

const YOSYS_LIB_NAMES_LINES: &str = include_str!("yosys_lib_names.txt");

fn yosys_lib_names() -> impl Iterator<Item = &'static str> {
    YOSYS_LIB_NAMES_LINES.split_terminator('\n')
}

fn main() {
    println!("cargo:rerun-if-env-changed=VTS_YOSYS_BUILD_DIR");
    println!("cargo:rerun-if-changed=yosys_lib_names.txt");
    println!("cargo:rerun-if-changed=CMakeLists.txt");
    let yosys_root = PathBuf::from("yosys");
    if yosys_root.is_dir() {
        println!("cargo:rerun-if-changed={}", yosys_root.display());
    }
    let vts_yosys_sys_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let yosys_build_dir = match env::var_os("VTS_YOSYS_BUILD_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => cmake::build(vts_yosys_sys_dir),
    };
    println!("cargo:rerun-if-changed=wrapper.cpp");
    cc::Build::new()
        .cpp(true)
        .file("wrapper.cpp")
        .compile("vts-yosys-wrapper");
    println!(
        "cargo:rustc-link-search=native={}/lib",
        yosys_build_dir.display()
    );
    for lib_name in yosys_lib_names() {
        println!("cargo:rustc-link-lib=static:+whole-archive={lib_name}");
    }
}
