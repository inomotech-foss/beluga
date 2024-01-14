use std::borrow::Cow;
use std::path::{Path, PathBuf};

use crate::{Builder, Context, SourceWithProperties};

pub fn run(builder: &mut Builder, include_dirs: &[Cow<Path>], enable_tracing: bool) {
    println!(
        "cargo:rerun-if-changed={}",
        builder.lib_dir.to_str().unwrap()
    );

    let lib_name = builder.lib_dir.file_name().unwrap().to_str().unwrap();
    let mut build = builder.cc_build.clone();
    build
        .warnings(true)
        .extra_warnings(false)
        .includes(include_dirs);

    if builder.aws_set_common_properties {
        builder
            .ctx
            .common_properties()
            .apply(&mut build, builder.ctx.profile, enable_tracing);
    }
    if builder.aws_set_thread_affinity_method {
        builder.ctx.thread_affinity_method().apply(&mut build);
    }
    if builder.aws_set_thread_name_method {
        builder.ctx.thread_name_method().apply(&mut build);
    }
    if builder.simd_add_definitions {
        builder.ctx.simd().apply_defines(&mut build);
    }

    let source_root_dir = builder.lib_dir.join("source");

    let mut prepared_objects = Vec::new();

    for src in &builder.sources_with_properties {
        let mut build = build.clone();
        prepared_objects.extend(compile_source_with_properties(
            builder.ctx,
            &mut build,
            &source_root_dir,
            src,
        ));
    }

    build_add_source(&mut build, &source_root_dir);
    for path in &builder.source_paths {
        build_add_source(&mut build, &source_root_dir.join(path));
    }
    for obj in prepared_objects {
        build.object(obj);
    }

    eprintln!("starting compilation");
    eprintln!("{builder:#?}");
    build.compile(lib_name);
}

fn compile_source_with_properties(
    ctx: &Context,
    build: &mut cc::Build,
    source_root_dir: &Path,
    src: &SourceWithProperties,
) -> Vec<PathBuf> {
    for path in &src.source_paths {
        build_add_source(build, &source_root_dir.join(path));
    }
    for flag in &src.compile_flags {
        build.flag(flag);
    }
    if src.simd_avx2 {
        ctx.simd().apply_flags(build);
    }
    build.compile_intermediates()
}

fn build_add_source(build: &mut cc::Build, path: &Path) {
    if path.is_file() {
        build.file(path);
        return;
    }

    let it = match path.read_dir() {
        Ok(v) => v,
        Err(err) => panic!("failed to read source directory {path:?}: {err}"),
    };
    for item in it {
        let item = item.unwrap();
        let file_type = item.file_type().unwrap();
        if !file_type.is_file() {
            continue;
        }
        let is_c_file = item
            .path()
            .extension()
            .map_or(false, |ext| ext.eq_ignore_ascii_case("c"));
        if !is_c_file {
            continue;
        }
        build.file(item.path());
    }
}
