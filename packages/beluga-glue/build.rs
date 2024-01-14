fn main() {
    let common_include = std::env::var_os("DEP_AWS_C_COMMON_INCLUDE").unwrap();

    println!("cargo:rerun-if-changed=src/logging.c");
    cc::Build::new()
        .warnings(true)
        .extra_warnings(true)
        .include(common_include)
        .file("src/logging.c")
        .compile("beluga-glue");
}
