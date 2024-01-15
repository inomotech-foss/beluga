use super::Profile;
use crate::Context;

/// Implementation of the `aws_set_common_properties` CMake function.
#[derive(Debug)]
pub struct CommonProperties {
    has_stdint: bool,
    has_stdbool: bool,
    has_wgnu: bool,
    no_gnu_expr: bool,
    have_sysconf: bool,
    compiler_specific: CompilerSpecific,
}

impl CommonProperties {
    pub fn detect(ctx: &Context) -> Self {
        eprintln!("detecting common properties");

        let has_stdint = super::check_include_file(ctx, "stdint.h");
        let has_stdbool = super::check_include_file(ctx, "stdbool.h");

        let has_wgnu = ctx
            .cc_build
            .is_flag_supported("-Wgnu")
            .expect("check -Wgnu compiler flag");
        let no_gnu_expr = if has_wgnu {
            // some platforms implement htonl family of functions via GNU statement expressions (https://gcc.gnu.org/onlinedocs/gcc/Statement-Exprs.html)
            // which generates -Wgnu-statement-expression warning.
            super::check_compiles_with_cc(
                ctx,
                ctx.cc_build.clone().flag("-Wgnu"),
                r#"
#include <netinet/in.h>

int main() {
    uint32_t x = 0;
    x = htonl(x);
    return (int)x;
}
"#,
            )
        } else {
            false
        };

        // some platforms (especially when cross-compiling) do not have the sysconf API
        // in their toolchain files.
        let have_sysconf = super::check_compiles(
            ctx,
            r#"
#include <unistd.h>
int main() { sysconf(_SC_NPROCESSORS_ONLN); }
"#,
        );
        Self {
            has_stdint,
            has_stdbool,
            has_wgnu,
            no_gnu_expr,
            have_sysconf,
            compiler_specific: CompilerSpecific::detect(ctx),
        }
    }

    pub fn apply(&self, build: &mut cc::Build, profile: Profile, enable_tracing: bool) {
        if !self.has_stdint {
            build.define("NO_STDINT", None);
        }
        if !self.has_stdbool {
            build.define("NO_STDBOOL", None);
        }
        if self.has_wgnu {
            build
                .flag("-Wgnu")
                .flag("-Wno-gnu-zero-variadic-macro-arguments");
        }
        if !self.no_gnu_expr {
            build.flag("-Wno-gnu-statement-expression");
        }
        if self.have_sysconf {
            build.define("HAVE_SYSCONF", None);
        }
        self.compiler_specific.apply(build);
        if matches!(profile, Profile::Debug) {
            build.define("DEBUG_BUILD", None);
        }
        if !enable_tracing {
            build.define("INTEL_NO_ITTNOTIFY_API", None);
        }
    }

    pub fn have_posix_large_file_support(&self) -> bool {
        match &self.compiler_specific {
            CompilerSpecific::Gnu { posix_lfs, .. } => posix_lfs.supported,
            _ => false,
        }
    }
}

#[derive(Debug)]
enum CompilerSpecific {
    Msvc,
    Gnu {
        outline_atomics: bool,
        posix_lfs: PosixLfs,
    },
}

impl CompilerSpecific {
    fn detect(ctx: &Context) -> Self {
        if ctx.compiler.is_like_msvc() {
            Self::Msvc
        } else {
            let outline_atomics = super::check_compiles_with_cc(
                ctx,
                ctx.cc_build
                    .clone()
                    .flag("-moutline-atomics")
                    .flag("-Werror"),
                r#"
int main() {
    int x = 1;
    __atomic_fetch_add(&x, -1, __ATOMIC_SEQ_CST);
    return x;
}
"#,
            );
            Self::Gnu {
                outline_atomics,
                posix_lfs: PosixLfs::detect(ctx),
            }
        }
    }

    fn apply(&self, build: &mut cc::Build) {
        match self {
            Self::Msvc => {
                build.flag("/volatile:iso").flag("/wd4204").flag("/wd4221");
            }
            Self::Gnu {
                outline_atomics,
                posix_lfs,
            } => {
                build
                    .flag("-Wstrict-prototypes")
                    .flag_if_supported("-fPIC")
                    .std("gnu99");
                if *outline_atomics {
                    build.flag("-moutline-atomics");
                }
                posix_lfs.apply(build);
            }
        }
    }
}

#[derive(Debug)]
struct PosixLfs {
    supported: bool,
    via_define: bool,
}

impl PosixLfs {
    fn detect(ctx: &Context) -> Self {
        const CODE: &str = r#"
#include <stdio.h>
/* fails to compile if off_t smaller than 64bits */
typedef char array[sizeof(off_t) >= 8 ? 1 : -1];
int main() { return 0; }
"#;

        let mut supported;
        let mut via_define = false;
        if super::check_compiles(ctx, CODE) {
            supported = true;
        } else if super::check_compiles_with_cc(
            ctx,
            ctx.cc_build.clone().define("_FILE_OFFSET_BITS", "64"),
            CODE,
        ) {
            supported = true;
            via_define = true;
        } else {
            supported = false;
        }

        if supported {
            // sometimes off_t is 64bit, but fseeko() is missing (ex: Android API < 24)
            supported = super::check_symbol_exists(ctx, ["stdio.h"], "fseeko");
        }

        Self {
            supported,
            via_define,
        }
    }

    fn apply(&self, build: &mut cc::Build) {
        if self.supported && self.via_define {
            build.define("_FILE_OFFSET_BITS", "64");
        }
    }
}
