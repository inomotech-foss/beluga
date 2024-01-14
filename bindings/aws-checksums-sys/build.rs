const USE_CPU_EXTENSIONS: bool = true;

fn main() {
    let ctx = aws_c_builder::Context::new();
    let mut builder = ctx.builder("aws-checksums");

    let mut use_generic = true;
    if USE_CPU_EXTENSIONS {
        if ctx.is_aws_arch_intel() {
            if ctx.aws_have_gcc_inline_asm() {
                builder.source_path("intel/asm");
                use_generic = false;
            } else if ctx.is_msvc() {
                builder.source_path("intel/visualc");
                use_generic = false;
            }
        } else if ctx.is_msvc() && ctx.is_aws_arch_arm64() {
            builder.source_path("arm");
            use_generic = false;
        } else if ctx.is_aws_arch_arm64() {
            builder
                .source_with_properties()
                .source_path("arm")
                .compile_flag("-march=armv8-a+crc");
            use_generic = false;
        } else if !ctx.is_msvc() && ctx.is_aws_arch_arm32() && check_arm32_crc(&ctx) {
            builder
                .source_with_properties()
                .source_path("arm")
                .source_path("arm/asm")
                .compile_flag("-march=armv8-a+crc");
            use_generic = false;
        }
    }

    if use_generic {
        builder.source_path("generic");
    }

    builder
        .aws_set_common_properties()
        .dependencies(["aws-c-common"])
        .build();
}

fn check_arm32_crc(ctx: &aws_c_builder::Context) -> bool {
    let mut build = ctx.get_cc_build();
    aws_c_builder::detect::check_compiles_with_cc(
        ctx,
        build.flag("-march=armv8-a+crc"),
        r"
#include <arm_acle.h>
int main() {
    int crc = __crc32d(0, 1);
    return 0;
}
",
    )
}
