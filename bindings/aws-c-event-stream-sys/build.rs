fn main() {
    let ctx = aws_c_builder::Context::new();
    ctx.builder("aws-c-event-stream")
        .aws_set_common_properties()
        .dependencies(["aws-c-io", "aws-c-common", "aws-checksums"])
        .build();
}
