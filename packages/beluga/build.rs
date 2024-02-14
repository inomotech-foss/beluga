use std::path::PathBuf;

fn main() {
    let c_iot_root = PathBuf::from(std::env::var("DEP_AWS_C_IOT_ROOT").unwrap());
    let crt_root = PathBuf::from(std::env::var("DEP_AWS_CRT_CPP_ROOT").unwrap());
    let c_common_root = PathBuf::from(std::env::var("DEP_AWS_C_COMMON_ROOT").unwrap());
    let c_io_root = PathBuf::from(std::env::var("DEP_AWS_C_IO_ROOT").unwrap());
    let c_compression_root = PathBuf::from(std::env::var("DEP_AWS_C_COMPRESSION_ROOT").unwrap());
    let c_cal_root = PathBuf::from(std::env::var("DEP_AWS_C_CAL_ROOT").unwrap());
    let c_sdk_utils_root = PathBuf::from(std::env::var("DEP_AWS_C_SDKUTILS_ROOT").unwrap());
    let c_s3_root = PathBuf::from(std::env::var("DEP_AWS_C_S3_ROOT").unwrap());
    let c_event_stream_root = PathBuf::from(std::env::var("DEP_AWS_C_EVENT_STREAM_ROOT").unwrap());
    let c_mqtt_root = PathBuf::from(std::env::var("DEP_AWS_C_MQTT_ROOT").unwrap());
    let c_http_root = PathBuf::from(std::env::var("DEP_AWS_C_HTTP_ROOT").unwrap());
    let c_auth_root = PathBuf::from(std::env::var("DEP_AWS_C_AUTH_ROOT").unwrap());
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
        .include(c_iot_root.join("include"))
        .include(crt_root.join("include"))
        .include(iot_device_sdk_root.join("include"))
        .include(c_common_root.join("include"))
        .include(c_io_root.join("include"))
        .include(c_compression_root.join("include"))
        .include(c_cal_root.join("include"))
        .include(c_sdk_utils_root.join("include"))
        .include(c_s3_root.join("include"))
        .include(c_event_stream_root.join("include"))
        .include(c_mqtt_root.join("include"))
        .include(c_http_root.join("include"))
        .include(c_auth_root.join("include"))
        .include("src/cpp")
        .files([
            "src/cpp/mqtt.cpp",
            "src/cpp/handle.cpp",
            "src/cpp/common.cpp",
            "src/cpp/tunnel.cpp",
            "src/cpp/jobs.cpp",
            "src/cpp/job.cpp",
        ])
        .compile("aws-sdk-wrapper");
}
