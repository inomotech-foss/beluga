fn main() {
    let ctx = aws_c_builder::Context::new();
    ctx.builder("aws-c-mqtt")
        .source_path("v5")
        .aws_set_common_properties()
        .dependencies([
            "aws-c-http",
            // transitive
            "aws-c-io",
            "aws-c-common",
        ])
        .build();
}
