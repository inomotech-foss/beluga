use std::path::Path;

const SYSTEM_INFO_FALLBACK: &str = "platform_fallback_stubs/system_info.c";

fn main() {
    let ctx = aws_c_builder::Context::new();

    let src_include_dir = Path::new("aws-c-common/include");
    let generated_include_dir = ctx.out_dir().join("include");
    prepare_headers(&ctx, src_include_dir, &generated_include_dir);

    let mut builder = ctx.builder("aws-c-common");
    builder
        .source_path("external")
        .set_include_dir(generated_include_dir)
        .bindings_suffix(determine_bindings_suffix(&ctx));

    if ctx.is_win32() {
        builder
            .source_path("windows")
            .source_path(SYSTEM_INFO_FALLBACK)
            .define("PSAPI_VERSION", "1");
    } else {
        builder.source_path("posix");

        if ctx.is_apple() {
            builder.source_path(SYSTEM_INFO_FALLBACK);
        } else if ctx.cmake_system_name().is_linux() {
            builder.source_path("linux");
        } else if ctx.cmake_system_name().is_bsd() {
            builder.source_path(SYSTEM_INFO_FALLBACK);
        } else if ctx.cmake_system_name().is_android() {
            builder
                .source_path("android")
                .source_path(SYSTEM_INFO_FALLBACK);
        } else {
            builder.source_path(SYSTEM_INFO_FALLBACK);
        }
    }

    // we always use cpu extensions
    if ctx.is_aws_arch_intel() {
        if ctx.is_msvc() {
            builder
                .source_path("arch/intel/cpuid.c")
                .source_path("arch/intel/msvc");
        } else {
            builder
                .source_path("arch/intel/cpuid.c")
                .source_path("arch/intel/asm");
        }
    } else if ctx.is_aws_arch_arm64() || ctx.is_aws_arch_arm32() {
        if ctx.is_msvc() {
            builder.source_path("arch/arm/msvc");
        } else {
            builder.source_path("arch/arm/asm");
        }
    } else {
        builder.source_path("arch/generic");
    }

    builder
        .aws_set_common_properties()
        .aws_set_thread_affinity_method()
        .aws_set_thread_name_method();

    builder.simd_add_definitions();
    if ctx.have_avx2_intrinsics() {
        builder.define("USE_SIMD_ENCODING", None);
        builder
            .source_with_properties()
            .source_path("arch/intel/encoding_avx2.c")
            .simd_avx2();
    }

    builder.build();
}

fn determine_bindings_suffix(ctx: &aws_c_builder::Context) -> &'static str {
    if ctx.is_apple() {
        "apple"
    } else if ctx.is_win32() {
        "windows"
    } else {
        "generic"
    }
}

fn prepare_headers(ctx: &aws_c_builder::Context, src_include_dir: &Path, out_include_dir: &Path) {
    aws_c_builder::c_header::copy_headers(src_include_dir, out_include_dir);
    let config_template = src_include_dir.join("aws/common/config.h.in");
    let generated_config_header = out_include_dir.join("aws/common/config.h");
    aws_c_builder::c_header::render_template(&config_template, &generated_config_header, |name| {
        match name {
            "AWS_HAVE_GCC_OVERFLOW_MATH_EXTENSIONS" => Some(false),
            "AWS_HAVE_GCC_INLINE_ASM" => Some(ctx.aws_have_gcc_inline_asm()),
            "AWS_HAVE_MSVC_INTRINSICS_X64" => Some(ctx.aws_have_msvc_intrinsics_x64()),
            "AWS_HAVE_POSIX_LARGE_FILE_SUPPORT" => Some(ctx.aws_have_posix_large_file_support()),
            "AWS_HAVE_EXECINFO" => Some(ctx.aws_have_execinfo()),
            "AWS_HAVE_WINAPI_DESKTOP" => Some(ctx.aws_have_winapi_desktop()),
            "AWS_HAVE_LINUX_IF_LINK_H" => Some(ctx.aws_have_linux_if_link_h()),
            _ => None,
        }
    });
}
