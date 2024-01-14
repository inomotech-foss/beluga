/* automatically generated by rust-bindgen 0.69.2 */

pub type aws_sdkutils_errors = ::core::ffi::c_uint;
pub type aws_sdkutils_log_subject = ::core::ffi::c_uint;
#[doc = " The profile specification has rule exceptions based on what file\n the profile collection comes from."]
pub type aws_profile_source_type = ::core::ffi::c_uint;
pub type aws_profile_section_type = ::core::ffi::c_uint;
pub type aws_endpoints_parameter_type = ::core::ffi::c_uint;
pub type aws_endpoints_resolved_endpoint_type = ::core::ffi::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct aws_profile_property {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct aws_profile {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct aws_profile_collection {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct aws_endpoints_ruleset {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct aws_partitions_config {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct aws_endpoints_parameter {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct aws_endpoints_rule_engine {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct aws_endpoints_resolved_endpoint {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct aws_endpoints_request_context {
    _unused: [u8; 0],
}
#[repr(C)]
pub struct aws_resource_name {
    pub partition: aws_byte_cursor,
    pub service: aws_byte_cursor,
    pub region: aws_byte_cursor,
    pub account_id: aws_byte_cursor,
    pub resource_id: aws_byte_cursor,
}
pub const AWS_C_SDKUTILS_PACKAGE_ID: u32 = 15;
pub const AWS_ERROR_SDKUTILS_GENERAL: aws_sdkutils_errors = 15360;
pub const AWS_ERROR_SDKUTILS_PARSE_FATAL: aws_sdkutils_errors = 15361;
pub const AWS_ERROR_SDKUTILS_PARSE_RECOVERABLE: aws_sdkutils_errors = 15362;
pub const AWS_ERROR_SDKUTILS_ENDPOINTS_UNSUPPORTED_RULESET: aws_sdkutils_errors = 15363;
pub const AWS_ERROR_SDKUTILS_ENDPOINTS_PARSE_FAILED: aws_sdkutils_errors = 15364;
pub const AWS_ERROR_SDKUTILS_ENDPOINTS_RESOLVE_INIT_FAILED: aws_sdkutils_errors = 15365;
pub const AWS_ERROR_SDKUTILS_ENDPOINTS_RESOLVE_FAILED: aws_sdkutils_errors = 15366;
pub const AWS_ERROR_SDKUTILS_ENDPOINTS_EMPTY_RULESET: aws_sdkutils_errors = 15367;
pub const AWS_ERROR_SDKUTILS_ENDPOINTS_RULESET_EXHAUSTED: aws_sdkutils_errors = 15368;
pub const AWS_ERROR_SDKUTILS_PARTITIONS_UNSUPPORTED: aws_sdkutils_errors = 15369;
pub const AWS_ERROR_SDKUTILS_PARTITIONS_PARSE_FAILED: aws_sdkutils_errors = 15370;
pub const AWS_ERROR_SDKUTILS_END_RANGE: aws_sdkutils_errors = 16383;
pub const AWS_LS_SDKUTILS_GENERAL: aws_sdkutils_log_subject = 15360;
pub const AWS_LS_SDKUTILS_PROFILE: aws_sdkutils_log_subject = 15361;
pub const AWS_LS_SDKUTILS_ENDPOINTS_PARSING: aws_sdkutils_log_subject = 15362;
pub const AWS_LS_SDKUTILS_ENDPOINTS_RESOLVE: aws_sdkutils_log_subject = 15363;
pub const AWS_LS_SDKUTILS_ENDPOINTS_GENERAL: aws_sdkutils_log_subject = 15364;
pub const AWS_LS_SDKUTILS_PARTITIONS_PARSING: aws_sdkutils_log_subject = 15365;
pub const AWS_LS_SDKUTILS_LAST: aws_sdkutils_log_subject = 16383;
pub const AWS_PST_NONE: aws_profile_source_type = 0;
pub const AWS_PST_CONFIG: aws_profile_source_type = 1;
pub const AWS_PST_CREDENTIALS: aws_profile_source_type = 2;
pub const AWS_PROFILE_SECTION_TYPE_PROFILE: aws_profile_section_type = 0;
pub const AWS_PROFILE_SECTION_TYPE_SSO_SESSION: aws_profile_section_type = 1;
pub const AWS_PROFILE_SECTION_TYPE_COUNT: aws_profile_section_type = 2;
pub const AWS_ENDPOINTS_PARAMETER_STRING: aws_endpoints_parameter_type = 0;
pub const AWS_ENDPOINTS_PARAMETER_BOOLEAN: aws_endpoints_parameter_type = 1;
pub const AWS_ENDPOINTS_RESOLVED_ENDPOINT: aws_endpoints_resolved_endpoint_type = 0;
pub const AWS_ENDPOINTS_RESOLVED_ERROR: aws_endpoints_resolved_endpoint_type = 1;
extern "C" {
    pub fn aws_sdkutils_library_init(allocator: *mut aws_allocator);
    pub fn aws_sdkutils_library_clean_up();
    #[doc = " Increments the reference count on the profile collection, allowing the caller to take a reference to it.\n\n Returns the same profile collection passed in."]
    pub fn aws_profile_collection_acquire(
        collection: *mut aws_profile_collection,
    ) -> *mut aws_profile_collection;
    #[doc = " Decrements a profile collection's ref count.  When the ref count drops to zero, the collection will be destroyed.\n Returns NULL."]
    pub fn aws_profile_collection_release(
        collection: *mut aws_profile_collection,
    ) -> *mut aws_profile_collection;
    #[doc = " @Deprecated This is equivalent to aws_profile_collection_release."]
    pub fn aws_profile_collection_destroy(profile_collection: *mut aws_profile_collection);
    #[doc = " Create a new profile collection by parsing a file with the specified path"]
    pub fn aws_profile_collection_new_from_file(
        allocator: *mut aws_allocator,
        file_path: *const aws_string,
        source: aws_profile_source_type,
    ) -> *mut aws_profile_collection;
    #[doc = " Create a new profile collection by merging a config-file-based profile\n collection and a credentials-file-based profile collection"]
    pub fn aws_profile_collection_new_from_merge(
        allocator: *mut aws_allocator,
        config_profiles: *const aws_profile_collection,
        credentials_profiles: *const aws_profile_collection,
    ) -> *mut aws_profile_collection;
    #[doc = " Create a new profile collection by parsing text in a buffer.  Primarily\n for testing."]
    pub fn aws_profile_collection_new_from_buffer(
        allocator: *mut aws_allocator,
        buffer: *const aws_byte_buf,
        source: aws_profile_source_type,
    ) -> *mut aws_profile_collection;
    #[doc = " Retrieves a reference to a profile with the specified name, if it exists, from the profile collection"]
    pub fn aws_profile_collection_get_profile(
        profile_collection: *const aws_profile_collection,
        profile_name: *const aws_string,
    ) -> *const aws_profile;
    pub fn aws_profile_collection_get_section(
        profile_collection: *const aws_profile_collection,
        section_type: aws_profile_section_type,
        section_name: *const aws_string,
    ) -> *const aws_profile;
    #[doc = " Returns the number of profiles in a collection"]
    pub fn aws_profile_collection_get_profile_count(
        profile_collection: *const aws_profile_collection,
    ) -> usize;
    #[doc = " Returns the number of elements of the specified section in a collection."]
    pub fn aws_profile_collection_get_section_count(
        profile_collection: *const aws_profile_collection,
        section_type: aws_profile_section_type,
    ) -> usize;
    #[doc = " Returns a reference to the name of the provided profile"]
    pub fn aws_profile_get_name(profile: *const aws_profile) -> *const aws_string;
    #[doc = " Retrieves a reference to a property with the specified name, if it exists, from a profile"]
    pub fn aws_profile_get_property(
        profile: *const aws_profile,
        property_name: *const aws_string,
    ) -> *const aws_profile_property;
    #[doc = " Returns how many properties a profile holds"]
    pub fn aws_profile_get_property_count(profile: *const aws_profile) -> usize;
    #[doc = " Returns a reference to the property's string value"]
    pub fn aws_profile_property_get_value(
        property: *const aws_profile_property,
    ) -> *const aws_string;
    #[doc = " Returns a reference to the value of a sub property with the given name, if it exists, in the property"]
    pub fn aws_profile_property_get_sub_property(
        property: *const aws_profile_property,
        sub_property_name: *const aws_string,
    ) -> *const aws_string;
    #[doc = " Returns how many sub properties the property holds"]
    pub fn aws_profile_property_get_sub_property_count(
        property: *const aws_profile_property,
    ) -> usize;
    #[doc = " Computes the final platform-specific path for the profile credentials file.  Does limited home directory\n expansion/resolution.\n\n override_path, if not null, will be searched first instead of using the standard home directory config path"]
    pub fn aws_get_credentials_file_path(
        allocator: *mut aws_allocator,
        override_path: *const aws_byte_cursor,
    ) -> *mut aws_string;
    #[doc = " Computes the final platform-specific path for the profile config file.  Does limited home directory\n expansion/resolution.\n\n override_path, if not null, will be searched first instead of using the standard home directory config path"]
    pub fn aws_get_config_file_path(
        allocator: *mut aws_allocator,
        override_path: *const aws_byte_cursor,
    ) -> *mut aws_string;
    #[doc = " Computes the profile to use for credentials lookups based on profile resolution rules"]
    pub fn aws_get_profile_name(
        allocator: *mut aws_allocator,
        override_name: *const aws_byte_cursor,
    ) -> *mut aws_string;
    pub fn aws_endpoints_get_supported_ruleset_version() -> aws_byte_cursor;
    pub fn aws_endpoints_parameter_get_type(
        parameter: *const aws_endpoints_parameter,
    ) -> aws_endpoints_parameter_type;
    pub fn aws_endpoints_parameter_get_built_in(
        parameter: *const aws_endpoints_parameter,
    ) -> aws_byte_cursor;
    pub fn aws_endpoints_parameter_get_default_string(
        parameter: *const aws_endpoints_parameter,
        out_cursor: *mut aws_byte_cursor,
    ) -> ::core::ffi::c_int;
    pub fn aws_endpoints_parameter_get_default_boolean(
        parameter: *const aws_endpoints_parameter,
        out_bool: *mut *const bool,
    ) -> ::core::ffi::c_int;
    pub fn aws_endpoints_parameter_get_is_required(
        parameter: *const aws_endpoints_parameter,
    ) -> bool;
    pub fn aws_endpoints_parameter_get_documentation(
        parameter: *const aws_endpoints_parameter,
    ) -> aws_byte_cursor;
    pub fn aws_endpoints_parameters_get_is_deprecated(
        parameter: *const aws_endpoints_parameter,
    ) -> bool;
    pub fn aws_endpoints_parameter_get_deprecated_message(
        parameter: *const aws_endpoints_parameter,
    ) -> aws_byte_cursor;
    pub fn aws_endpoints_parameter_get_deprecated_since(
        parameter: *const aws_endpoints_parameter,
    ) -> aws_byte_cursor;
    pub fn aws_endpoints_ruleset_new_from_string(
        allocator: *mut aws_allocator,
        ruleset_json: aws_byte_cursor,
    ) -> *mut aws_endpoints_ruleset;
    pub fn aws_endpoints_ruleset_acquire(
        ruleset: *mut aws_endpoints_ruleset,
    ) -> *mut aws_endpoints_ruleset;
    pub fn aws_endpoints_ruleset_release(
        ruleset: *mut aws_endpoints_ruleset,
    ) -> *mut aws_endpoints_ruleset;
    pub fn aws_endpoints_ruleset_get_parameters(
        ruleset: *mut aws_endpoints_ruleset,
    ) -> *const aws_hash_table;
    pub fn aws_endpoints_ruleset_get_version(
        ruleset: *const aws_endpoints_ruleset,
    ) -> aws_byte_cursor;
    pub fn aws_endpoints_ruleset_get_service_id(
        ruleset: *const aws_endpoints_ruleset,
    ) -> aws_byte_cursor;
    #[doc = " Create new rule engine for a given ruleset.\n In cases of failure NULL is returned and last error is set."]
    pub fn aws_endpoints_rule_engine_new(
        allocator: *mut aws_allocator,
        ruleset: *mut aws_endpoints_ruleset,
        partitions_config: *mut aws_partitions_config,
    ) -> *mut aws_endpoints_rule_engine;
    pub fn aws_endpoints_rule_engine_acquire(
        rule_engine: *mut aws_endpoints_rule_engine,
    ) -> *mut aws_endpoints_rule_engine;
    pub fn aws_endpoints_rule_engine_release(
        rule_engine: *mut aws_endpoints_rule_engine,
    ) -> *mut aws_endpoints_rule_engine;
    pub fn aws_endpoints_request_context_new(
        allocator: *mut aws_allocator,
    ) -> *mut aws_endpoints_request_context;
    pub fn aws_endpoints_request_context_acquire(
        request_context: *mut aws_endpoints_request_context,
    ) -> *mut aws_endpoints_request_context;
    pub fn aws_endpoints_request_context_release(
        request_context: *mut aws_endpoints_request_context,
    ) -> *mut aws_endpoints_request_context;
    pub fn aws_endpoints_request_context_add_string(
        allocator: *mut aws_allocator,
        context: *mut aws_endpoints_request_context,
        name: aws_byte_cursor,
        value: aws_byte_cursor,
    ) -> ::core::ffi::c_int;
    pub fn aws_endpoints_request_context_add_boolean(
        allocator: *mut aws_allocator,
        context: *mut aws_endpoints_request_context,
        name: aws_byte_cursor,
        value: bool,
    ) -> ::core::ffi::c_int;
    pub fn aws_endpoints_rule_engine_resolve(
        engine: *mut aws_endpoints_rule_engine,
        context: *const aws_endpoints_request_context,
        out_resolved_endpoint: *mut *mut aws_endpoints_resolved_endpoint,
    ) -> ::core::ffi::c_int;
    pub fn aws_endpoints_resolved_endpoint_acquire(
        resolved_endpoint: *mut aws_endpoints_resolved_endpoint,
    ) -> *mut aws_endpoints_resolved_endpoint;
    pub fn aws_endpoints_resolved_endpoint_release(
        resolved_endpoint: *mut aws_endpoints_resolved_endpoint,
    ) -> *mut aws_endpoints_resolved_endpoint;
    pub fn aws_endpoints_resolved_endpoint_get_type(
        resolved_endpoint: *const aws_endpoints_resolved_endpoint,
    ) -> aws_endpoints_resolved_endpoint_type;
    pub fn aws_endpoints_resolved_endpoint_get_url(
        resolved_endpoint: *const aws_endpoints_resolved_endpoint,
        out_url: *mut aws_byte_cursor,
    ) -> ::core::ffi::c_int;
    pub fn aws_endpoints_resolved_endpoint_get_properties(
        resolved_endpoint: *const aws_endpoints_resolved_endpoint,
        out_properties: *mut aws_byte_cursor,
    ) -> ::core::ffi::c_int;
    pub fn aws_endpoints_resolved_endpoint_get_headers(
        resolved_endpoint: *const aws_endpoints_resolved_endpoint,
        out_headers: *mut *const aws_hash_table,
    ) -> ::core::ffi::c_int;
    pub fn aws_endpoints_resolved_endpoint_get_error(
        resolved_endpoint: *const aws_endpoints_resolved_endpoint,
        out_error: *mut aws_byte_cursor,
    ) -> ::core::ffi::c_int;
    pub fn aws_partitions_get_supported_version() -> aws_byte_cursor;
    pub fn aws_partitions_config_new_from_string(
        allocator: *mut aws_allocator,
        json: aws_byte_cursor,
    ) -> *mut aws_partitions_config;
    pub fn aws_partitions_config_acquire(
        partitions: *mut aws_partitions_config,
    ) -> *mut aws_partitions_config;
    pub fn aws_partitions_config_release(
        partitions: *mut aws_partitions_config,
    ) -> *mut aws_partitions_config;
    #[doc = "Given an ARN \"Amazon Resource Name\" represented as an in memory a\nstructure representing the parts"]
    pub fn aws_resource_name_init_from_cur(
        arn: *mut aws_resource_name,
        input: *const aws_byte_cursor,
    ) -> ::core::ffi::c_int;
    #[doc = "Calculates the space needed to write an ARN to a byte buf"]
    pub fn aws_resource_name_length(
        arn: *const aws_resource_name,
        size: *mut usize,
    ) -> ::core::ffi::c_int;
    #[doc = "Serializes an ARN structure into the lexical string format"]
    pub fn aws_byte_buf_append_resource_name(
        buf: *mut aws_byte_buf,
        arn: *const aws_resource_name,
    ) -> ::core::ffi::c_int;
}
