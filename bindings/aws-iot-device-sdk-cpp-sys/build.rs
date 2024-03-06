fn main() {
    aws_c_builder::Config::new("aws-iot-device-sdk-cpp-v2")
        .aws_dependencies(["AWS_CRT_CPP", "AWS_C_IOT"])
        .link_libraries([
            "Discovery-cpp",
            "EventstreamRpc-cpp",
            "GreengrassIpc-cpp",
            "IotDeviceCommon-cpp",
            "IotDeviceDefender-cpp",
            "IotIdentity-cpp",
            "IotJobs-cpp",
            "IotSecureTunneling-cpp",
            "IotShadow-cpp",
        ])
        .run_bindgen(true)
        .bindgen_callback(|builder| {
            builder
                .allowlist_recursively(false)
                .allowlist_type("Aws::Iotjobs::JobStatus")
                .rustified_enum("Aws::Iotjobs::JobStatus")
                .allowlist_type("Aws::Iotjobs::RejectedErrorCode")
                .rustified_enum("Aws::Iotjobs::RejectedErrorCode")
                .allowlist_recursively(true)
                .clang_args(["-x", "c++"])
                .detect_include_paths(true)
                .enable_cxx_namespaces()
        })
        .build();
}
