fn main() {
    aws_c_builder::Config::new("aws-c-mqtt")
        .aws_dependencies(["AWS_C_HTTP"])
        .bindgen_callback(|builder| {
            builder
                .allowlist_item("(?i)aws_(c_)?mqtt.*")
                .allowlist_type("on_connection_closed_data")
        })
        .build();
}
