fn main() {
    aws_c_builder::Config::new("aws-c-s3")
        .aws_dependencies([
            "AWS_C_AUTH",
            "AWS_C_COMMON",
            "AWS_C_HTTP",
            "AWS_C_IO",
            "AWS_C_SDKUTILS",
            "AWS_CHECKSUMS",
        ])
        .bindgen_callback(|builder| {
            builder
                .allowlist_item("(?i)aws_(c_)?s3.*")
                .allowlist_type("aws_credentials_properties_s3express")
        })
        .build();
}
