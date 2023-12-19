fn main() {
    let root_paths = aws_c_builder::get_dependency_root_paths(&["AWS_IOT_DEVICE_SDK_CPP_V2"]);
    let dependency_includes = root_paths.into_iter().map(|path| format!("{path}/include"));

    println!("cargo:rerun-if-changed=src/cpp");
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .warnings_into_errors(true)
        .extra_warnings(true)
        .warnings(true)
        .flag_if_supported("-Wlogical-op")
        .flag_if_supported("-Wfloat-equal")
        .flag_if_supported("-Wno-attributes")
        .flag_if_supported("-pedantic")
        .includes(dependency_includes)
        .include("src/cpp")
        .files([
            "src/cpp/mqtt.cpp",
            "src/cpp/handle.cpp",
            "src/cpp/common.cpp",
            "src/cpp/logs.cpp",
        ])
        .compile("aws-sdk-wrapper");
}
