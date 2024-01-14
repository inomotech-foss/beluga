use crate::Context;

/// Implements content of AwsFeatureTests.cmake
#[derive(Debug)]
pub struct FeatureTests {
    pub have_winapi_desktop: bool,
    pub have_gcc_inline_asm: bool,
    pub have_auxv: bool,
    pub have_execinfo: bool,
    pub have_linux_if_link_h: bool,
    pub have_msvc_intrinsics_x64: bool,
}

impl FeatureTests {
    pub fn detect(ctx: &Context) -> Self {
        eprintln!("running feature tests");
        let have_winapi_desktop = super::check_compiles(
            ctx,
            r#"
#include <Windows.h>
#if WINAPI_FAMILY_PARTITION(WINAPI_PARTITION_DESKTOP)
int main() { return 0; }
#else
it's not windows desktop
#endif
"#,
        );
        let have_gcc_inline_asm = super::check_compiles(
            ctx,
            r#"
int main() {
    int foo = 42, bar = 24;
    __asm__ __volatile__("":"=r"(foo):"r"(bar):"memory");
}
"#,
        );
        let have_auxv = super::check_compiles(
            ctx,
            r#"
#include <sys/auxv.h>
int main() {
#ifdef __linux__
    getauxval(AT_HWCAP);
    getauxval(AT_HWCAP2);
#endif
    return 0;
}
"#,
        );
        let have_execinfo = super::check_compiles(
            ctx,
            r#"
#include <execinfo.h>
#include <stdlib.h>
int main() {
    backtrace(NULL, 0);
    return 0;
}
"#,
        );
        let have_linux_if_link_h = super::check_include_file(ctx, "linux/if_link.h");
        let have_msvc_intrinsics_x64 = if ctx.compiler.is_like_msvc() {
            super::check_compiles(
                ctx,
                r#"
#include <intrin.h>
int main() {
    unsigned __int64 a = 0x0fffffffffffffffI64;
    unsigned __int64 b = 0xf0000000I64;
    unsigned __int64 c, d;
    d = _umul128(a, b, &c);
    return 0;
}
    "#,
            )
        } else {
            false
        };

        Self {
            have_winapi_desktop,
            have_gcc_inline_asm,
            have_auxv,
            have_execinfo,
            have_linux_if_link_h,
            have_msvc_intrinsics_x64,
        }
    }
}
