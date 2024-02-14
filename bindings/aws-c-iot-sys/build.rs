fn main() {
    aws_c_builder::Config::new("aws-c-iot")
        .aws_dependencies(["AWS_C_MQTT"])
        .bindgen_callback(|builder| {
            builder
                .allowlist_item("(?i)aws_(c_)?iotdevice.*")
                .allowlist_item("(?i)aws_secure_tunnel.*")
        })
        .build();
}
