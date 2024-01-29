fn main() {
    aws_c_builder::Config::new("aws-c-compression")
        .aws_dependencies(["AWS_C_COMMON"])
        .bindgen_callback(|builder| {
            builder
                .allowlist_item("(?i)aws_(c_)?compression.*")
                .allowlist_item("(?i)aws_huffman.*")
        })
        .build();
}
