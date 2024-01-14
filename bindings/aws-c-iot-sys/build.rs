fn main() {
    let ctx = aws_c_builder::Context::new();
    ctx.builder("aws-c-iot")
        .aws_set_common_properties()
        .dependencies([
            "aws-c-mqtt",
            // transitive
            "aws-c-common",
            "aws-c-http",
            "aws-c-io",
        ])
        .build();
}
