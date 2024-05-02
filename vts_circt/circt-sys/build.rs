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
    const CIRCT_INSTALL_DIR: &str = "CIRCT_SYS_CIRCT_BUILD_DIR";
    let watch_envs = [CIRCT_DIR, CIRCT_INSTALL_DIR];
    for env in watch_envs {
        println!("cargo:rerun-if-env-changed={env}");
    }

    let cargo_manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let circt_dir = env::var(CIRCT_DIR)
        .map(PathBuf::from)
        .unwrap_or(cargo_manifest_dir.join("circt"));
    ensure_exists(&circt_dir);

    let circt_install_dir = env::var(CIRCT_INSTALL_DIR)
        .map(PathBuf::from)
        .unwrap_or(cargo_manifest_dir.join("install"));
    ensure_exists(&circt_install_dir);

    let watch_dirs = [&circt_dir, &circt_install_dir];
    for dir in watch_dirs {
        println!("cargo:rerun-if-changed={}", dir.display());
    }

    let llvm_config = LLVMConfig::from(circt_install_dir.join("bin/llvm-config"));
    let llvm_lib_dir = llvm_config.option("libdir");
    println!("cargo:rustc-link-search=native={llvm_lib_dir}");
    let llvm_include_dir = llvm_config.option("includedir");

    let link_libs = [
        "CIRCTHW",
        "CIRCTHWToLLHD",
        "CIRCTLLHD",
        "LLVMDemangle",
        "LLVMSupport",
        "MLIRAnalysis",
        "MLIRControlFlowInterfaces",
        "MLIRInferTypeOpInterface",
        "MLIRIR",
        "MLIRFunctionInterfaces",
        "MLIRPass",
        "MLIRPDLDialect",
        "MLIRPDLInterpDialect",
        "MLIRPDLToPDLInterp",
        "MLIRRewrite",
        "MLIRRewritePDL",
        "MLIRSideEffectInterfaces",
        "MLIRSupport",
        "MLIRTransforms",
        "MLIRTransformUtils",
    ];
    for lib in link_libs {
        println!("cargo:rustc-link-lib=static={lib}");
    }

    let system_libs = llvm_config.option("system-libs");
    let iter_system_libs = system_libs
        .split_whitespace()
        .map(|lib| lib.strip_prefix("-l").unwrap_or(lib));
    for lib in iter_system_libs {
        println!("cargo:rustc-link-lib=dylib={lib}")
    }

    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=wrapper.cpp");

    cc::Build::new()
        .include(llvm_include_dir)
        .file("wrapper.cpp")
        .cpp(true)
        .warnings(false)
        .extra_warnings(false)
        .compile("circt-sys-wrapper")
}
