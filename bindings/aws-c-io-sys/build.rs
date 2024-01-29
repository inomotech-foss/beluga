use std::path::{Path, PathBuf};

fn main() {
    let cmake_import_dir;
    let mut config = aws_c_builder::Config::new("aws-c-io");

    if aws_c_builder::is_linux_like() {
        let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
        cmake_import_dir = out_dir.join("_cmake_imports");
        std::fs::create_dir_all(&cmake_import_dir).unwrap();

        create_s2n_package_config(&cmake_import_dir).unwrap();
        config = config.extra_cmake_prefix_paths([cmake_import_dir.to_str().unwrap()]);
    }

    config
        .aws_dependencies(["AWS_C_CAL", "AWS_C_COMMON"])
        .bindgen_callback(|builder| {
            builder
                .allowlist_item("(?i)aws_async_input_stream.*")
                .allowlist_item("(?i)aws_channel.*")
                .allowlist_item("(?i)aws_client_bootstrap.*")
                .allowlist_item("(?i)aws_custom_key_op_handler.*")
                .allowlist_item("(?i)aws_event_loop.*")
                .allowlist_item("(?i)aws_exponential_backoff.*")
                .allowlist_item("(?i)aws_future.*")
                .allowlist_item("(?i)aws_host.*")
                .allowlist_item("(?i)aws_input_stream.*")
                .allowlist_item("(?i)aws_(c_)?io.*")
                .allowlist_item("(?i)aws_pkcs11_lib.*")
                .allowlist_item("(?i)aws_retry.*")
                .allowlist_item("(?i)aws_server.*")
                .allowlist_item("(?i)aws_socket.*")
                .allowlist_item("(?i)aws_stream.*")
                .allowlist_item("(?i)aws_tls.*")
                .allowlist_type("aws_address_record_type")
                .allowlist_type("aws_generate_random_fn")
                .allowlist_type("aws_new_event_loop_fn")
                .allowlist_type("aws_standard_retry_options")
                .allowlist_type("aws.+host.+_fn")
        })
        .build();
}

fn create_s2n_package_config(target_dir: &Path) -> anyhow::Result<()> {
    use std::fmt::Write;

    let s2n_include = std::env::var("DEP_S2N_TLS_INCLUDE")?;

    let mut content = String::new();
    writeln!(&mut content, "if (NOT TARGET AWS::s2n)")?;
    writeln!(&mut content, "add_library(AWS::s2n STATIC IMPORTED)")?;
    writeln!(
        &mut content,
        "target_include_directories(AWS::s2n INTERFACE \"{s2n_include}\")"
    )?;
    writeln!(&mut content, "endif()")?;
    std::fs::write(target_dir.join("s2n-config.cmake"), content)?;
    Ok(())
}
