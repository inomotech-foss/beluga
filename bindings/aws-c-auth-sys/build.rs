fn main() {
    aws_c_builder::Config::new("aws-c-auth")
        .aws_dependencies(["AWS_C_COMMON", "AWS_C_HTTP", "AWS_C_SDKUTILS"])
        .bindgen_callback(|builder| {
            builder
                .allowlist_item("(?i)aws_(c_)?auth.*")
                .allowlist_item("(?i)aws_cognito.*")
                .allowlist_item("(?i)aws_credentials.*")
                .allowlist_item("(?i)aws_imds.*")
                .allowlist_item("(?i)aws_sign.*")
                .allowlist_type("aws_should_sign_header_fn")
                .allowlist_type("aws.+credentials.+_fn")
        })
        .build();
}
