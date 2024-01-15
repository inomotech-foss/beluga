fn main() {
    let ctx = aws_c_builder::Context::new();
    let mut builder = ctx.builder("aws-c-iot");
    if ctx.is_win32() {
        builder.source_path("windows");
    } else if ctx.is_apple() {
        builder.source_path("apple");
    } else if ctx.is_unix() {
        builder.source_path("linux");
    }

    builder
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
