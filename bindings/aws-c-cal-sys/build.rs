fn main() {
    let ctx = aws_c_builder::Context::new();
    let mut builder = ctx.builder("aws-c-cal");
    if ctx.is_win32() {
        builder.source_path("windows");
    } else if ctx.is_apple() {
        builder.source_path("darwin");
    } else {
        builder.source_path("unix");
    }

    // TODO: separate bindings for ios because of
    // aws_ecc_key_pair_new_generate_random
    builder
        .aws_set_common_properties()
        .dependencies(["aws-c-common"])
        .build();
}
