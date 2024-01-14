use std::path::Path;

pub fn copy_headers(src_include_dir: &Path, out_include_dir: &Path) {
    for entry in src_include_dir.read_dir().expect("read include dir") {
        let entry = entry.unwrap();
        let file_type = entry.file_type().unwrap();
        let path = entry.path();
        let rel_path = path.strip_prefix(src_include_dir).unwrap();
        if file_type.is_dir() {
            let target_dir = out_include_dir.join(rel_path);
            std::fs::create_dir_all(&target_dir).unwrap();
            copy_headers(&path, &target_dir);
            continue;
        }
        let is_header = file_type.is_file()
            && path.extension().is_some_and(|ext| {
                ext.eq_ignore_ascii_case("h") || ext.eq_ignore_ascii_case("inl")
            });
        if !is_header {
            continue;
        }

        std::fs::copy(&path, out_include_dir.join(rel_path)).unwrap();
    }
}

pub fn render_template(src: &Path, out: &Path, get_define_value: impl Fn(&str) -> Option<bool>) {
    let template = std::fs::read_to_string(src).expect("read config header template");
    let output = render_template_content(&template, get_define_value);
    std::fs::write(out, output).expect("write config header");
}

/// Renders a cmake configuration header file by replacing all the config
/// template lines.
///
/// See: <https://cmake.org/cmake/help/latest/command/configure_file.html>
fn render_template_content(
    template: &str,
    get_define_value: impl Fn(&str) -> Option<bool>,
) -> String {
    const DEFINE_MARKER: &str = "#cmakedefine ";

    let mut output = String::with_capacity(template.len());
    for line in template.lines() {
        if let Some(define_args) = line.strip_prefix(DEFINE_MARKER) {
            match render_template_define(define_args, &get_define_value) {
                Ok(line) => output.push_str(&line),
                Err(err) => panic!("{err}\nline: {line:?}"),
            }
        } else {
            output.push_str(line);
        }
        output.push('\n');
    }
    output
}

/// Renders a single #cmakedefine line.
///
/// Defines with values (ex. `#cmakedefine FOO_STRING "@FOO_STRING@"`) are
/// not supported.
///
/// Syntax: <https://cmake.org/cmake/help/latest/command/configure_file.html>
fn render_template_define(
    define_args: &str,
    get_define_value: impl Fn(&str) -> Option<bool>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut define_args = define_args.split_whitespace();
    let define_name = define_args.next().ok_or("missing VAR name")?;
    if define_args.next().is_some() {
        return Err("extended #cmakedefine not supported".into());
    }

    let value = get_define_value(define_name)
        .ok_or_else(|| format!("unknown define name: {define_name:?}"))?;
    if value {
        Ok(format!("#define {define_name}"))
    } else {
        Ok(format!("/* #undef {define_name} */"))
    }
}
