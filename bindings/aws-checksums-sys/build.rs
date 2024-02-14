fn main() {
    aws_c_builder::Config::new("aws-checksums")
        .aws_dependencies(["AWS_C_COMMON"])
        .bindgen_callback(|builder| builder.allowlist_item("(?i)aws_checksums.*"))
        .build();
}
