fn main() {
    aws_c_builder::Config::new("aws-c-event-stream")
        .aws_dependencies(["AWS_C_COMMON", "AWS_C_IO", "AWS_CHECKSUMS"])
        .bindgen_callback(|builder| builder.allowlist_item("(?i)aws_(c_)?event_stream.*"))
        .build();
}
