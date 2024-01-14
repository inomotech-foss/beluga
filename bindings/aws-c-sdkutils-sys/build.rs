fn main() {
    let ctx = aws_c_builder::Context::new();
    ctx.builder("aws-c-sdkutils")
        .aws_set_common_properties()
        .dependencies(["aws-c-common"])
        .build();
}
