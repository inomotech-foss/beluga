use std::borrow::Cow;
use std::cell::OnceCell;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub use cc;

use self::detect::{
    CommonProperties, FeatureTests, Profile, Simd, TargetArch, TargetFamily, TargetVendor,
    ThreadAffinityMethod, ThreadNameMethod,
};
use crate::detect::TargetOs;
pub use crate::to_cow::ToCow;

mod bindings;
pub mod c_header;
mod compile;
pub mod detect;
mod to_cow;

const ENABLE_TRACING_FEATURE: &str = "enable-tracing";

#[derive(Debug)]
pub struct Context {
    out_dir: PathBuf,
    cc_build: cc::Build,
    compiler: cc::Tool,
    profile: Profile,
    target_arch: TargetArch,
    target_family: TargetFamily,
    target_vendor: TargetVendor,
    target_os: TargetOs,
    common_properties: OnceCell<CommonProperties>,
    feature_tests: OnceCell<FeatureTests>,
    simd: OnceCell<Simd>,
    thread_affinity_method: OnceCell<ThreadAffinityMethod>,
    thread_name_method: OnceCell<ThreadNameMethod>,
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

impl Context {
    pub fn new() -> Self {
        let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
        let cc_build = cc::Build::new();
        let compiler = cc_build.get_compiler();
        Self {
            out_dir,
            cc_build,
            compiler,
            profile: Profile::from_env(),
            target_arch: TargetArch::from_env(),
            target_family: TargetFamily::from_env(),
            target_vendor: TargetVendor::from_env(),
            target_os: TargetOs::from_env(),
            common_properties: OnceCell::new(),
            feature_tests: OnceCell::new(),
            simd: OnceCell::new(),
            thread_affinity_method: OnceCell::new(),
            thread_name_method: OnceCell::new(),
        }
    }

    pub fn out_dir(&self) -> &Path {
        &self.out_dir
    }

    pub fn get_cc_build(&self) -> cc::Build {
        self.cc_build.clone()
    }

    pub fn builder<'a>(&'a self, lib_dir: impl ToCow<'a, Path>) -> Builder<'a> {
        Builder::new(self, lib_dir.to_cow())
    }

    // compiler info

    pub fn is_msvc(&self) -> bool {
        self.compiler.is_like_msvc()
    }

    // target info

    pub fn is_win32(&self) -> bool {
        matches!(self.target_family, TargetFamily::Windows)
    }

    pub fn is_apple(&self) -> bool {
        matches!(self.target_vendor, TargetVendor::Apple)
    }

    pub fn cmake_system_name(&self) -> CMakeSystemName {
        CMakeSystemName {
            target_os: self.target_os,
        }
    }

    pub fn is_aws_arch_intel(&self) -> bool {
        matches!(self.target_arch, TargetArch::X86 | TargetArch::X86_64)
    }

    pub fn is_aws_arch_arm64(&self) -> bool {
        matches!(self.target_arch, TargetArch::Aarch64)
    }

    pub fn is_aws_arch_arm32(&self) -> bool {
        matches!(self.target_arch, TargetArch::Arm)
    }

    // detect
    // TODO: caching by loading from common?

    fn common_properties(&self) -> &CommonProperties {
        self.common_properties
            .get_or_init(|| CommonProperties::detect(self))
    }

    fn feature_tests(&self) -> &FeatureTests {
        self.feature_tests
            .get_or_init(|| FeatureTests::detect(self))
    }

    fn simd(&self) -> &Simd {
        self.simd.get_or_init(|| Simd::detect(self))
    }

    fn thread_affinity_method(&self) -> &ThreadAffinityMethod {
        self.thread_affinity_method
            .get_or_init(|| ThreadAffinityMethod::detect(self))
    }

    fn thread_name_method(&self) -> &ThreadNameMethod {
        self.thread_name_method
            .get_or_init(|| ThreadNameMethod::detect(self))
    }

    // feature tests

    pub fn aws_have_gcc_inline_asm(&self) -> bool {
        self.feature_tests().have_gcc_inline_asm
    }

    pub fn aws_have_msvc_intrinsics_x64(&self) -> bool {
        self.feature_tests().have_msvc_intrinsics_x64
    }

    pub fn aws_have_posix_large_file_support(&self) -> bool {
        self.common_properties().have_posix_large_file_support()
    }

    pub fn aws_have_execinfo(&self) -> bool {
        self.feature_tests().have_execinfo
    }

    pub fn aws_have_winapi_desktop(&self) -> bool {
        self.feature_tests().have_winapi_desktop
    }

    pub fn aws_have_linux_if_link_h(&self) -> bool {
        self.feature_tests().have_linux_if_link_h
    }

    // simd

    pub fn have_avx2_intrinsics(&self) -> bool {
        self.simd().have_avx2_intrinsics
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CMakeSystemName {
    target_os: TargetOs,
}

impl CMakeSystemName {
    pub fn is_linux(&self) -> bool {
        matches!(self.target_os, TargetOs::Linux)
    }

    pub fn is_bsd(&self) -> bool {
        self.target_os.is_bsd()
    }

    pub fn is_android(&self) -> bool {
        matches!(self.target_os, TargetOs::Android)
    }
}

#[derive(Debug)]
pub struct Builder<'a> {
    ctx: &'a Context,
    lib_dir: Cow<'a, Path>,
    cc_build: cc::Build,
    dependencies: Vec<Cow<'a, str>>,
    include_dir: Option<Cow<'a, Path>>,
    bindings_suffix: Option<Cow<'a, str>>,
    source_paths: Vec<Cow<'a, Path>>,
    sources_with_properties: Vec<SourceWithProperties<'a>>,
    aws_set_common_properties: bool,
    aws_set_thread_affinity_method: bool,
    aws_set_thread_name_method: bool,
    simd_add_definitions: bool,
}

impl<'a> Builder<'a> {
    // meta

    fn new(ctx: &'a Context, lib_dir: Cow<'a, Path>) -> Self {
        let cc_build = ctx.cc_build.clone();
        Self {
            ctx,
            lib_dir,
            cc_build,
            dependencies: Vec::new(),
            include_dir: None,
            bindings_suffix: None,
            source_paths: Vec::new(),
            sources_with_properties: Vec::new(),
            aws_set_common_properties: false,
            aws_set_thread_affinity_method: false,
            aws_set_thread_name_method: false,
            simd_add_definitions: false,
        }
    }

    pub fn set_include_dir(&mut self, path: impl ToCow<'a, Path>) -> &mut Self {
        self.include_dir = Some(path.to_cow());
        self
    }

    pub fn bindings_suffix(&mut self, suffix: impl ToCow<'a, str>) -> &mut Self {
        self.bindings_suffix = Some(suffix.to_cow());
        self
    }

    pub fn dependencies<It>(&mut self, deps: It) -> &mut Self
    where
        It: IntoIterator,
        It::Item: ToCow<'a, str>,
    {
        self.dependencies
            .extend(deps.into_iter().map(|x| x.to_cow()));
        self
    }

    // source

    pub fn source_path(&mut self, path: impl ToCow<'a, Path>) -> &mut Self {
        self.source_paths.push(path.to_cow());
        self
    }

    pub fn source_with_properties(&mut self) -> &mut SourceWithProperties<'a> {
        self.sources_with_properties
            .push(SourceWithProperties::new());
        self.sources_with_properties.last_mut().unwrap()
    }

    pub fn define(&mut self, var: &str, val: impl Into<Option<&'a str>>) -> &mut Self {
        self.cc_build.define(var, val);
        self
    }

    pub fn build(&mut self) {
        let include_dir = self
            .include_dir
            .clone()
            .unwrap_or_else(|| Cow::Owned(self.lib_dir.join("include")));
        println!(
            "cargo:include={}",
            include_dir.canonicalize().unwrap().to_str().unwrap()
        );

        let include_dirs = std::iter::once(include_dir)
            .chain(
                self.dependencies
                    .iter()
                    .map(|name| get_dependency_include_path(name))
                    .map(Cow::Owned),
            )
            .collect::<Vec<_>>();

        let enable_tracing = is_feature_enabled(ENABLE_TRACING_FEATURE);
        crate::compile::run(self, &include_dirs, enable_tracing);
        crate::bindings::prepare(
            &self.ctx.out_dir,
            &include_dirs,
            self.bindings_suffix.as_deref().unwrap_or(""),
        )
    }

    // aws specific

    pub fn simd_add_definitions(&mut self) -> &mut Self {
        self.simd_add_definitions = true;
        self
    }

    pub fn aws_set_common_properties(&mut self) -> &mut Self {
        self.aws_set_common_properties = true;
        self
    }

    pub fn aws_set_thread_affinity_method(&mut self) -> &mut Self {
        self.aws_set_thread_affinity_method = true;
        self
    }

    pub fn aws_set_thread_name_method(&mut self) -> &mut Self {
        self.aws_set_thread_name_method = true;
        self
    }
}

#[derive(Debug)]
pub struct SourceWithProperties<'a> {
    source_paths: Vec<Cow<'a, Path>>,
    compile_flags: Vec<Cow<'a, str>>,
    simd_avx2: bool,
}

impl<'a> SourceWithProperties<'a> {
    fn new() -> Self {
        Self {
            source_paths: Vec::new(),
            compile_flags: Vec::new(),
            simd_avx2: false,
        }
    }

    pub fn source_path(&mut self, path: impl ToCow<'a, Path>) -> &mut Self {
        self.source_paths.push(path.to_cow());
        self
    }

    pub fn compile_flag(&mut self, flag: impl ToCow<'a, str>) -> &mut Self {
        self.compile_flags.push(flag.to_cow());
        self
    }

    /// Simulates `simd_add_source_avx2`
    pub fn simd_avx2(&mut self) -> &mut Self {
        self.simd_avx2 = true;
        self
    }
}

fn get_dependency_include_path(dependency: &str) -> PathBuf {
    PathBuf::from(get_dependency_variable_os(dependency, "include"))
}

fn get_dependency_variable_os(dependency: &str, name: &str) -> OsString {
    match try_get_dependency_variable_os(dependency, name) {
        Some(v) => v,
        None => panic!("dependency {dependency:?} didn't set the variable {name:?}"),
    }
}

fn try_get_dependency_variable_os(dependency: &str, name: &str) -> Option<OsString> {
    let env_name = format!(
        "DEP_{}_{}",
        dependency.replace('-', "_").to_ascii_uppercase(),
        name.to_ascii_uppercase()
    );
    std::env::var_os(env_name)
}

fn is_feature_enabled(name: &str) -> bool {
    std::env::var_os(format!(
        "CARGO_FEATURE_{}",
        name.replace('-', "_").to_ascii_uppercase()
    ))
    .is_some()
}
