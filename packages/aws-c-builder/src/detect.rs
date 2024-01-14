use std::sync::atomic::AtomicU8;

pub use self::cflags::CommonProperties;
pub use self::feature_tests::FeatureTests;
pub use self::simd::Simd;
pub use self::thread_affinity::ThreadAffinityMethod;
pub use self::thread_name::ThreadNameMethod;
use crate::Context;

mod cflags;
mod feature_tests;
mod simd;
mod thread_affinity;
mod thread_name;

/// Checks whether the given code snippet successfully compiles.
///
/// Must not be called in parallel.
pub fn check_compiles(ctx: &Context, code: &str) -> bool {
    check_compiles_with_cc(ctx, &mut ctx.cc_build.clone(), code)
}

/// Checks whether the given code snippet successfully compiles with a
/// pre-configured [`cc::Build`].
///
/// Must not be called in parallel.
pub fn check_compiles_with_cc(ctx: &Context, build: &mut cc::Build, code: &str) -> bool {
    let out_dir = ctx.out_dir.join("comptest");
    std::fs::create_dir_all(&out_dir).expect("create comptest dir");

    let c_file = {
        // we want at least some way to investigate compilation issues, but it also
        // doesn't need to be as complicated as taking the hash from the source code
        static ID: AtomicU8 = AtomicU8::new(0);
        let id = ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        out_dir.join(format!("test_{id:06X}.c"))
    };

    std::fs::write(&c_file, code).expect("write c code compilation test code");
    // TODO: this is too noisy
    build
        .cargo_metadata(false)
        .emit_rerun_if_env_changed(false)
        .warnings(false)
        .extra_warnings(false)
        .opt_level(0)
        .out_dir(out_dir)
        .file(c_file)
        .try_compile_intermediates()
        .is_ok()
}

/// Checks whether a given symbol is available during compilation of C code.
///
/// Based on cmake's implementation.
/// See: <https://github.com/Kitware/CMake/blob/master/Modules/CheckSymbolExists.cmake>
pub fn check_symbol_exists<H>(ctx: &Context, headers: H, symbol: &str) -> bool
where
    H: IntoIterator,
    H::Item: AsRef<str>,
{
    use std::fmt::Write;
    let mut code = String::new();
    for header in headers {
        writeln!(code, "#include <{}>", header.as_ref()).unwrap();
    }
    write!(
        code,
        "
int main(int argc, char** argv) {{
    (void)argv;
    #ifndef {symbol}
    return ((int*)(&{symbol}))[argc];
    #else
    (void)argc;
    return 0;
    #endif
}}
"
    )
    .unwrap();
    check_compiles(ctx, &code)
}

/// Checks whether a given header is available during compilation.
///
/// See: <https://github.com/Kitware/CMake/blob/master/Modules/CheckIncludeFile.cmake>
pub fn check_include_file(ctx: &Context, name: &str) -> bool {
    let code = format!(
        r#"
#include <{name}>
int main(void) {{ return 0; }}
"#
    );
    check_compiles(ctx, &code)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    Debug,
    Release,
}

impl Profile {
    pub fn from_env() -> Self {
        match std::env::var("PROFILE").as_deref() {
            Ok("debug") => Self::Debug,
            _ => Self::Release,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetFamily {
    Other,
    Unix,
    Windows,
}

impl TargetFamily {
    pub fn from_env() -> Self {
        match std::env::var("CARGO_CFG_TARGET_FAMILY").as_deref() {
            Ok("unix") => Self::Unix,
            Ok("windows") => Self::Windows,
            _ => Self::Other,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetOs {
    Other,
    Windows,
    Macos,
    Ios,
    Linux,
    Android,
    Freebsd,
    Dragonfly,
    Openbsd,
    Netbsd,
}

impl TargetOs {
    pub fn from_env() -> Self {
        match std::env::var("CARGO_CFG_TARGET_OS").as_deref() {
            Ok("windows") => Self::Windows,
            Ok("macos") => Self::Macos,
            Ok("ios") => Self::Ios,
            Ok("linux") => Self::Linux,
            Ok("android") => Self::Android,
            Ok("freebsd") => Self::Freebsd,
            Ok("dragonfly") => Self::Dragonfly,
            Ok("openbsd") => Self::Openbsd,
            Ok("netbsd") => Self::Netbsd,
            _ => Self::Other,
        }
    }

    pub const fn is_bsd(self) -> bool {
        matches!(self, Self::Freebsd | Self::Openbsd | Self::Netbsd)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetVendor {
    Other,
    Apple,
}

impl TargetVendor {
    pub fn from_env() -> Self {
        match std::env::var("CARGO_CFG_TARGET_VENDOR").as_deref() {
            Ok("apple") => Self::Apple,
            _ => Self::Other,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetArch {
    Other,
    X86,
    X86_64,
    Arm,
    Aarch64,
}

impl TargetArch {
    pub fn from_env() -> Self {
        match std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
            Ok("x86") => Self::X86,
            Ok("x86_64") => Self::X86_64,
            Ok("arm") => Self::Arm,
            Ok("aarch64") => Self::Aarch64,
            _ => Self::Other,
        }
    }
}
