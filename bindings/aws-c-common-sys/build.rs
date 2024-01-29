fn main() {
    aws_c_builder::Config::new("aws-c-common")
        .bindgen_blanket_include_dirs(["common"])
        .bindgen_callback(|builder| {
            builder
                .blocklist_item("aws_format_standard_log_line")
                .blocklist_item("aws_log_formatter_format_fn")
                .blocklist_item("(__)?CFAllocator.*")
                .opaque_type("aws_hash_table")
                .opaque_type("aws_log_formatter_vtable")
                .opaque_type("aws_thread_once")
        })
        .build();
}
