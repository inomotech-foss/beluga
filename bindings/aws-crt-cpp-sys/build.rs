fn main() {
    aws_c_builder::Config::new("aws-crt-cpp")
        .aws_dependencies([
            "AWS_C_AUTH",
            "AWS_C_CAL",
            "AWS_C_COMMON",
            "AWS_C_EVENT_STREAM",
            "AWS_C_HTTP",
            "AWS_C_IO",
            "AWS_C_MQTT",
            "AWS_C_S3",
            "AWS_CHECKSUMS",
        ])
        .run_bindgen(true)
        .bindgen_callback(|builder| {
            builder
                .allowlist_recursively(true)
                .allowlist_type("Aws::Crt::DateTime")
                .allowlist_type("Aws::Crt::UUID")
                .rustified_enum("Aws::Crt::DayOfWeek")
                .rustified_enum("Aws::Crt::Month")
                .rustified_enum("Aws::Crt::DateFormat")
                .opaque_type("Aws::Crt::Optional")
                .opaque_type("Aws::Crt::String")
                .opaque_type("Aws::Crt::StringStream")
                .opaque_type("Aws::Crt::Vector")
                .opaque_type("Aws::Crt::List")
                .clang_args(["-x", "c++"])
                .detect_include_paths(true)
                .generate_cstr(true)
                .enable_cxx_namespaces()
        })
        .build();
}
