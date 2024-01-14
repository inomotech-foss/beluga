use std::borrow::Cow;
use std::path::Path;

pub fn prepare(out_dir: &Path, include_dirs: &[Cow<Path>], bindings_suffix: &str) {
    let bindings_dir = Path::new("bindings");
    let bindings_file = bindings_dir.join(get_bindings_file_name(bindings_suffix));
    println!("cargo:rerun-if-changed={}", bindings_file.to_str().unwrap());

    #[cfg(feature = "generate-bindings")]
    generate(bindings_dir, &bindings_file, include_dirs);
    let _ = include_dirs;

    if let Err(err) = std::fs::copy(bindings_file, out_dir.join("bindings.rs")) {
        eprintln!("HINT: try enabling the 'generate-bindings' feature with '--features aws-c-builder/generate-bindings' to generate the bindings.");
        panic!("failed to copy bindings to target directory: {err}");
    };
}

fn get_bindings_file_name(suffix: &str) -> String {
    const PREFIX: &str = "wrapper";
    if suffix.is_empty() {
        format!("{PREFIX}.rs")
    } else {
        format!("{PREFIX}_{suffix}.rs")
    }
}

#[cfg(feature = "generate-bindings")]
fn generate(bindings_dir: &Path, bindings_file: &Path, include_dirs: &[Cow<Path>]) {
    let wrapper_header = bindings_dir.join("wrapper.h");
    let wrapper_items = bindings_dir.join("wrapper.items");
    for path in [&wrapper_header, &wrapper_items] {
        println!("cargo:rerun-if-changed={}", path.to_str().unwrap());
    }

    let include_args = include_dirs
        .iter()
        .map(|path| format!("-I{}", path.to_str().unwrap()));

    let mut builder = bindgen::builder()
        .allowlist_recursively(false)
        .array_pointers_in_arguments(true)
        .enable_function_attribute_detection()
        .formatter(bindgen::Formatter::Rustfmt)
        .generate_cstr(true)
        .layout_tests(false)
        .merge_extern_blocks(true)
        .prepend_enum_name(false)
        .sort_semantically(true)
        .use_core()
        .clang_args(include_args)
        .header(wrapper_header.to_str().unwrap());
    builder = load_allowlist_items(builder, &wrapper_items);
    eprintln!("bindgen builder: {builder:?}");
    let bindings = builder.generate().expect("generate bindings");
    bindings
        .write_to_file(bindings_file)
        .expect("write bindings");
}

#[cfg(feature = "generate-bindings")]
fn load_allowlist_items(mut builder: bindgen::Builder, wrapper_items: &Path) -> bindgen::Builder {
    let content = std::fs::read_to_string(wrapper_items).expect("read allowlist");
    for line in content.lines() {
        if line.is_empty() || line.starts_with("# ") {
            continue;
        }

        if let Some(line) = line.strip_prefix("block:") {
            builder = builder.blocklist_item(line);
            continue;
        }
        if let Some(line) = line.strip_prefix("opaque:") {
            builder = builder.opaque_type(line);
            continue;
        }
        let line = line.strip_prefix("allow:").unwrap_or(line);
        builder = builder.allowlist_item(line);
    }
    builder
}
