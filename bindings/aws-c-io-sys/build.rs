fn main() {
    let ctx = aws_c_builder::Context::new();
    let mut builder = ctx.builder("aws-c-io");
    builder.bindings_suffix(determine_bindings_suffix(&ctx));

    let mut use_s2n = false;
    let event_loop_define;

    if ctx.is_win32() {
        builder.source_path("windows/iocp");
        event_loop_define = "IO_COMPLETION_PORTS";
    } else if ctx.cmake_system_name().is_linux() || ctx.cmake_system_name().is_android() {
        event_loop_define = "EPOLL";
        use_s2n = true;
    } else if ctx.is_apple() {
        builder
            .source_path("bsd")
            .source_path("posix")
            .source_path("darwin");
        event_loop_define = "KQUEUE";
    } else if ctx.cmake_system_name().is_bsd() {
        builder.source_path("bsd").source_path("posix");
        event_loop_define = "KQUEUE";
        use_s2n = true;
    } else {
        event_loop_define = "";
    }

    if use_s2n {
        builder.source_path("s2n");
    }

    builder.aws_set_common_properties();
    if !event_loop_define.is_empty() {
        builder.define(&format!("AWS_USE_{event_loop_define}"), None);
    }

    if use_s2n {
        builder.define("USE_S2N", None).dependencies(["s2n-tls"]);
    }

    builder.dependencies(["aws-c-common", "aws-c-cal"]).build();
}

fn determine_bindings_suffix(ctx: &aws_c_builder::Context) -> &'static str {
    if ctx.is_win32() {
        "win32_iocp"
    } else if ctx.is_apple() {
        "apple"
    } else {
        "generic"
    }
}
