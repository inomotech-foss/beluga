//! Build script dependency for all related aws c library packages.

use std::path::PathBuf;

type CmakeCallbackFn<'a> = Box<dyn FnOnce(&mut cmake::Config) + 'a>;
type BindgenCallbackFn<'a> = Box<dyn FnOnce(bindgen::Builder) -> bindgen::Builder + 'a>;

pub struct Config<'a> {
    lib_name: &'a str,
    aws_dependencies: Vec<&'a str>,
    link_libraries: Vec<&'a str>,
    bindgen_blanket_include_dirs: Vec<&'a str>,
    extra_cmake_prefix_paths: Vec<&'a str>,
    cmake_callback: Option<CmakeCallbackFn<'a>>,
    run_bindgen: bool,
    bindgen_callback: Option<BindgenCallbackFn<'a>>,
}

impl<'a> Config<'a> {
    pub fn new(lib_name: &'a str) -> Self {
        Self {
            lib_name,
            aws_dependencies: Vec::new(),
            link_libraries: Vec::new(),
            bindgen_blanket_include_dirs: Vec::new(),
            extra_cmake_prefix_paths: Vec::new(),
            cmake_callback: None,
            run_bindgen: true,
            bindgen_callback: None,
        }
    }

    pub fn aws_dependencies(mut self, deps: impl IntoIterator<Item = &'a str>) -> Self {
        self.aws_dependencies.extend(deps);
        self
    }

    pub fn link_libraries(mut self, libs: impl IntoIterator<Item = &'a str>) -> Self {
        self.link_libraries.extend(libs);
        self
    }

    pub fn bindgen_blanket_include_dirs(
        mut self,
        names: impl IntoIterator<Item = &'a str>,
    ) -> Self {
        self.bindgen_blanket_include_dirs.extend(names);
        self
    }

    pub fn extra_cmake_prefix_paths(mut self, paths: impl IntoIterator<Item = &'a str>) -> Self {
        self.extra_cmake_prefix_paths.extend(paths);
        self
    }

    pub fn cmake_callback(mut self, callback: impl FnOnce(&mut cmake::Config) + 'a) -> Self {
        self.cmake_callback = Some(Box::new(callback));
        self
    }

    pub fn run_bindgen(mut self, doit: bool) -> Self {
        self.run_bindgen = doit;
        self
    }

    pub fn bindgen_callback(
        mut self,
        callback: impl FnOnce(bindgen::Builder) -> bindgen::Builder + 'a,
    ) -> Self {
        self.bindgen_callback = Some(Box::new(callback));
        self
    }

    pub fn build(mut self) {
        let dependency_root_paths = get_dependency_root_paths(self.aws_dependencies.clone());
        println!("DEPENDENCY PATHS {dependency_root_paths:?}");
        let cmake_prefix_path = dependency_root_paths
            .iter()
            .map(String::as_str)
            .chain(self.extra_cmake_prefix_paths.iter().copied())
            .collect::<Vec<_>>()
            .join(";");
        println!("cargo:cmake_prefix_path={cmake_prefix_path}");
        let out_dir = self.compile(&cmake_prefix_path);
        if self.run_bindgen {
            self.generate_bindings(&out_dir, &dependency_root_paths);
        }
    }

    fn compile(&mut self, cmake_prefix_path: &str) -> String {
        println!("cargo:rerun-if-changed={}", self.lib_name);
        let mut config = cmake::Config::new(self.lib_name);
        config
            .define("CMAKE_PREFIX_PATH", cmake_prefix_path)
            .define("AWS_ENABLE_LTO", "ON")
            .define("BUILD_DEPS", "OFF")
            .define("BUILD_TESTING", "OFF");

        if let Some(cb) = self.cmake_callback.take() {
            cb(&mut config);
        }

        let out_dir = config.build().to_str().unwrap().to_owned();
        println!("cargo:rustc-link-search=native={out_dir}/lib");
        if self.link_libraries.is_empty() {
            self.link_libraries.push(self.lib_name);
        }
        for name in &self.link_libraries {
            println!("cargo:rustc-link-lib=static={name}");
        }
        out_dir
    }

    fn generate_bindings(&mut self, lib_root: &str, dependency_root_paths: &[String]) {
        let include_args = std::iter::once(lib_root)
            .chain(dependency_root_paths.iter().map(String::as_str))
            .map(|path| format!("-I{path}/include"));

        println!("cargo:rerun-if-changed=wrapper.h");
        let mut builder = bindgen::builder()
            .allowlist_recursively(false)
            .array_pointers_in_arguments(true)
            .enable_function_attribute_detection()
            .generate_cstr(true)
            .merge_extern_blocks(true)
            .prepend_enum_name(false)
            .sort_semantically(true)
            .use_core()
            .clang_args(include_args)
            .header("wrapper.h");
        if let Some(cb) = self.bindgen_callback.take() {
            builder = cb(builder);
        }
        for name in &self.bindgen_blanket_include_dirs {
            builder = builder.allowlist_file(format!(".*/{name}/[^/]+\\.h"));
        }

        let bindings = builder.generate().unwrap();

        let out_path = PathBuf::from(std::env::var("OUT_DIR").unwrap());
        bindings
            .write_to_file(out_path.join("bindings.rs"))
            .unwrap();
    }
}

pub fn get_dependency_root_paths<'a>(deps: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let deps = deps.into_iter();

    let mut all_paths = Vec::<String>::with_capacity({
        let (lower, upper) = deps.size_hint();
        upper.unwrap_or(lower)
    });
    for dep in deps {
        let root = get_build_variable(dep, "ROOT");
        if all_paths.iter().any(|existing| existing == &root) {
            // since it's transitive, we know that we have all its dependencies as well.
            continue;
        }

        all_paths.push(root);
        all_paths.extend(
            get_build_variable(dep, "CMAKE_PREFIX_PATH")
                .split(';')
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned),
        );
        all_paths.sort_unstable();
        all_paths.dedup();
    }

    all_paths
}

fn get_build_variable(package: &str, var: &str) -> String {
    let Ok(v) = std::env::var(format!("DEP_{package}_{var}")) else {
        panic!("package '{package}' didn't set the '{var}' variable in its build script or isn't a direct dependency of this package");
    };
    v
}

#[must_use]
pub fn is_linux_like() -> bool {
    // anything unix that isn't macos is considered "Linux" by AWS
    std::env::var("CARGO_CFG_TARGET_FAMILY").unwrap() == "unix"
        && std::env::var("CARGO_CFG_TARGET_OS").unwrap() != "macos"
}
