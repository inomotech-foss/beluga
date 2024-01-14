fn main() {
    let ctx = aws_c_builder::Context::new();
    ctx.builder("aws-c-auth")
        .dependencies([
            "aws-c-cal",
            "aws-c-common",
            "aws-c-http",
            "aws-c-io",
            "aws-c-sdkutils",
        ])
        .aws_set_common_properties()
        .build();
}
