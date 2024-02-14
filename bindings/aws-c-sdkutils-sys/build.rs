fn main() {
    aws_c_builder::Config::new("aws-c-sdkutils")
        .aws_dependencies(["AWS_C_COMMON"])
        .bindgen_callback(|builder| {
            builder
                .allowlist_item("(?i)aws_endpoints.*")
                .allowlist_item("(?i)aws_partitions.*")
                .allowlist_item("(?i)aws_profile.*")
                .allowlist_item("(?i)aws_resource_name.*")
                .allowlist_item("(?i)aws_(c_)?sdkutils.*")
        })
        .build();
}
