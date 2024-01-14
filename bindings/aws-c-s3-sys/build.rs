fn main() {
    let ctx = aws_c_builder::Context::new();
    ctx.builder("aws-c-s3")
        .aws_set_common_properties()
        .dependencies([
            "aws-c-auth",
            "aws-checksums",
            // transitive
            "aws-c-cal",
            "aws-c-common",
            "aws-c-http",
            "aws-c-io",
            "aws-c-sdkutils",
        ])
        .build();
}
