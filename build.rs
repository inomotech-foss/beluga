use std::path::PathBuf;

fn main() {
    let crt_root = PathBuf::from(std::env::var("DEP_AWS_CRT_CPP_ROOT").unwrap());
    let iot_device_sdk_root =
        PathBuf::from(std::env::var("DEP_AWS_IOT_DEVICE_SDK_CPP_V2_ROOT").unwrap());

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
        .include(crt_root.join("include"))
        .include(iot_device_sdk_root.join("include"))
        .include("src/cpp")
        .files([
            "src/cpp/mqtt.cpp",
            "src/cpp/handle.cpp",
            "src/cpp/common.cpp",
        ])
        .compile("aws-sdk-wrapper");
}
