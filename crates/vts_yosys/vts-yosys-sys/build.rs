fn main() {
    println!("cargo:rerun-if-changed=wrapper.cpp");
    cc::Build::new()
        .cpp(true)
        .file("wrapper.cpp")
        .compile("vts-yosys-wrapper");
}
