use std::path::{Path, PathBuf};

fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let cmake_import_dir;
    let mut config = aws_c_builder::Config::new("aws-c-cal");

    if aws_c_builder::is_linux_like() {
        cmake_import_dir = out_dir.join("_cmake_imports");
        std::fs::create_dir_all(&cmake_import_dir).unwrap();

        create_crypto_package_config(&cmake_import_dir).unwrap();
        config = config.extra_cmake_prefix_paths([cmake_import_dir.to_str().unwrap()]);
    }

    config
        .aws_dependencies(["AWS_C_COMMON"])
        .bindgen_callback(|builder| {
            builder
                .allowlist_item("(?i)aws_(c_)?cal.*")
                .allowlist_item("(?i)aws_ecc.*")
                .allowlist_item("(?i)aws_hash.*")
                .allowlist_item("(?i)aws_hmac.*")
                .allowlist_item("(?i)aws_rsa.*")
                .allowlist_item("(?i)aws_symmetric.*")
                .allowlist_recursively(true)
        })
        .build();

    // we need to remove this file so it doesn't take precedence over our
    // crypto-config.cmake file.
    std::fs::remove_file(out_dir.join("lib/aws-c-cal/cmake/modules/Findcrypto.cmake")).unwrap();
}

fn create_crypto_package_config(target_dir: &Path) -> anyhow::Result<()> {
    use std::fmt::Write;

    let aws_lc_root = std::env::var("DEP_AWS_LC_0_13_3_ROOT")?;

    let mut content = String::new();
    writeln!(&mut content, "if (NOT TARGET AWS::crypto)")?;
    writeln!(&mut content, "add_library(AWS::crypto STATIC IMPORTED)")?;
    writeln!(
        &mut content,
        "target_include_directories(AWS::crypto INTERFACE \"{aws_lc_root}/include\")"
    )?;
    writeln!(&mut content, "endif()")?;
    std::fs::write(target_dir.join("crypto-config.cmake"), content)?;

    Ok(())
}
