use std::env;
use std::io;
use std::path::PathBuf;
use std::process::{self, Command};

fn ensure_exists(path: &PathBuf) {
    if !path.exists() {
        panic!(r#""{}" does not exist"#, path.display())
    }
}

struct LLVMConfig(PathBuf);

impl From<PathBuf> for LLVMConfig {
    fn from(path: PathBuf) -> Self {
        Self(path)
    }
}

impl LLVMConfig {
    fn run(&self, args: &[&str]) -> io::Result<process::Output> {
        Command::new(&self.0).args(args).output()
    }

    fn option(&self, name: &'static str) -> String {
        let name = format!("--{name}");
        self.run(&[name.as_str()])
            .map(|output| {
                if !output.status.success() {
                    println!(
                        r#"failed to get option "{name}": "{}""#,
                        String::from_utf8(output.stderr.clone()).expect("should be valid utf-8")
                    );
                }
                String::from_utf8(output.stdout.clone())
                    .expect("should be valid utf-8")
                    .lines()
                    .next()
                    .expect("should be single line")
                    .to_string()
            })
            .map_err(|err| {
                println!(r#"I/O error occurred: {err}"#);
            })
            .unwrap()
    }
}

fn main() {
    const CIRCT_DIR: &str = "CIRCT_SYS_CIRCT_DIR";
    const CIRCT_BUILD_DIR: &str = "CIRCT_SYS_CIRCT_BUILD_DIR";
    const LLVM_DIR: &str = "CIRCT_SYS_LLVM_DIR";
    const LLVM_BUILD_DIR: &str = "CIRCT_SYS_LLVM_BUILD_DIR";
    let watch_envs = [CIRCT_DIR, CIRCT_BUILD_DIR, LLVM_DIR, LLVM_BUILD_DIR];
    for env in watch_envs {
        println!("cargo:rerun-if-env-changed={env}");
    }

    let cargo_manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let circt_dir = env::var(CIRCT_DIR)
        .map(PathBuf::from)
        .unwrap_or(cargo_manifest_dir.join("circt"));
    ensure_exists(&circt_dir);

    let circt_build_dir = env::var(CIRCT_BUILD_DIR)
        .map(PathBuf::from)
        .unwrap_or(circt_dir.join("build"));
    ensure_exists(&circt_build_dir);

    let llvm_dir = env::var(LLVM_DIR)
        .map(PathBuf::from)
        .unwrap_or(circt_dir.join("llvm"));
    ensure_exists(&llvm_dir);

    let llvm_build_dir = env::var(LLVM_BUILD_DIR)
        .map(PathBuf::from)
        .unwrap_or(llvm_dir.join("build"));
    ensure_exists(&llvm_build_dir);

    let watch_dirs = [&circt_dir, &circt_build_dir, &llvm_dir, &llvm_build_dir];
    for dir in watch_dirs {
        println!("cargo:rerun-if-changed={}", dir.display());
    }

    let llvm_config = LLVMConfig::from(llvm_build_dir.join("bin/llvm-config"));
    let llvm_lib_dir = llvm_config.option("libdir");
    println!("cargo:rustc-link-search=native={llvm_lib_dir}");
    let llvm_include_dir = llvm_config.option("includedir");

    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=wrapper.cpp");

    cc::Build::new()
        .include(llvm_include_dir)
        .file("wrapper.cpp")
        .warnings(false)
        .extra_warnings(false)
        .compile("circt-sys-wrapper")
}
