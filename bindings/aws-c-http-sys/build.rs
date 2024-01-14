fn main() {
    let ctx = aws_c_builder::Context::new();
    ctx.builder("aws-c-http")
        .aws_set_common_properties()
        .dependencies([
            "aws-c-io",
            "aws-c-compression",
            // transitive
            "aws-c-common",
            "aws-c-cal",
        ])
        .build();
}
