use aws_c_auth_sys::{
    AWS_LS_AUTH_CREDENTIALS_PROVIDER, AWS_LS_AUTH_PROFILE, AWS_LS_AUTH_SIGNING, AWS_LS_IMDS_CLIENT,
};
use aws_c_cal_sys::{
    AWS_LS_CAL_DER, AWS_LS_CAL_ECC, AWS_LS_CAL_HASH, AWS_LS_CAL_HMAC, AWS_LS_CAL_LIBCRYPTO_RESOLVE,
    AWS_LS_CAL_RSA,
};
use aws_c_common_sys::{
    AWS_LS_COMMON_BUS, AWS_LS_COMMON_IO, AWS_LS_COMMON_JSON_PARSER, AWS_LS_COMMON_MEMTRACE,
    AWS_LS_COMMON_TASK_SCHEDULER, AWS_LS_COMMON_TEST, AWS_LS_COMMON_THREAD,
    AWS_LS_COMMON_XML_PARSER,
};
use aws_c_event_stream_sys::{
    AWS_LS_EVENT_STREAM_CHANNEL_HANDLER, AWS_LS_EVENT_STREAM_RPC_CLIENT,
    AWS_LS_EVENT_STREAM_RPC_SERVER,
};
use aws_c_http_sys::{
    AWS_LS_HTTP_CONNECTION, AWS_LS_HTTP_CONNECTION_MANAGER, AWS_LS_HTTP_DECODER,
    AWS_LS_HTTP_ENCODER, AWS_LS_HTTP_PROXY_NEGOTIATION, AWS_LS_HTTP_SERVER, AWS_LS_HTTP_STREAM,
    AWS_LS_HTTP_STREAM_MANAGER, AWS_LS_HTTP_WEBSOCKET, AWS_LS_HTTP_WEBSOCKET_SETUP,
};
use aws_c_io_sys::{
    AWS_LS_IO_ALPN, AWS_LS_IO_CHANNEL, AWS_LS_IO_CHANNEL_BOOTSTRAP, AWS_LS_IO_DNS,
    AWS_LS_IO_EVENT_LOOP, AWS_LS_IO_EXPONENTIAL_BACKOFF_RETRY_STRATEGY, AWS_LS_IO_FILE_UTILS,
    AWS_LS_IO_PEM, AWS_LS_IO_PKCS11, AWS_LS_IO_PKI, AWS_LS_IO_SHARED_LIBRARY, AWS_LS_IO_SOCKET,
    AWS_LS_IO_SOCKET_HANDLER, AWS_LS_IO_STANDARD_RETRY_STRATEGY, AWS_LS_IO_TLS,
};
use aws_c_iot_sys::{
    AWS_LS_IOTDEVICE_DEFENDER_TASK, AWS_LS_IOTDEVICE_DEFENDER_TASK_CONFIG,
    AWS_LS_IOTDEVICE_NETWORK_CONFIG, AWS_LS_IOTDEVICE_SECURE_TUNNELING,
};
use aws_c_mqtt_sys::{
    AWS_LS_MQTT5_CANARY, AWS_LS_MQTT5_CLIENT, AWS_LS_MQTT5_GENERAL, AWS_LS_MQTT5_TO_MQTT3_ADAPTER,
    AWS_LS_MQTT_CLIENT, AWS_LS_MQTT_TOPIC_TREE,
};
use aws_c_s3_sys::{
    AWS_LS_S3_CLIENT, AWS_LS_S3_CLIENT_STATS, AWS_LS_S3_ENDPOINT, AWS_LS_S3_META_REQUEST,
    AWS_LS_S3_REQUEST,
};
use aws_c_sdkutils_sys::{
    AWS_LS_SDKUTILS_ENDPOINTS_GENERAL, AWS_LS_SDKUTILS_ENDPOINTS_PARSING,
    AWS_LS_SDKUTILS_ENDPOINTS_RESOLVE, AWS_LS_SDKUTILS_PARTITIONS_PARSING, AWS_LS_SDKUTILS_PROFILE,
};

use super::{PackageId, Subject};

impl Subject {
    pub const fn static_target(self) -> Option<&'static str> {
        macro_rules! tree {
            (
                match $self:ident {
                    $(
                        $package_id:ident as $package:literal {
                            $($value:ident => $override:literal,)*
                        }
                    )+
                }
            ) => {
                match $self.package_id() {
                    $(
                        PackageId::$package_id => match $self.0 {
                            $(
                                $value => Some(concat!("aws::", $package, "::", $override)),
                            )*
                            _ => Some(concat!("aws::", $package)),
                        },
                    )+
                    _ => None,
                }
            };
        }

        tree!(
            match self {
                COMMON as "common" {
                    AWS_LS_COMMON_TASK_SCHEDULER => "task_scheduler",
                    AWS_LS_COMMON_THREAD => "thread",
                    AWS_LS_COMMON_MEMTRACE => "memtrace",
                    AWS_LS_COMMON_XML_PARSER => "xml_parser",
                    AWS_LS_COMMON_IO => "io",
                    AWS_LS_COMMON_BUS => "bus",
                    AWS_LS_COMMON_TEST => "test",
                    AWS_LS_COMMON_JSON_PARSER => "json_parser",
                }
                IO as "io" {
                    AWS_LS_IO_EVENT_LOOP => "event_loop",
                    AWS_LS_IO_SOCKET => "socket",
                    AWS_LS_IO_SOCKET_HANDLER => "socket_handler",
                    AWS_LS_IO_TLS => "tls",
                    AWS_LS_IO_ALPN => "alpn",
                    AWS_LS_IO_DNS => "dns",
                    AWS_LS_IO_PKI => "pki",
                    AWS_LS_IO_CHANNEL => "channel",
                    AWS_LS_IO_CHANNEL_BOOTSTRAP => "channel_bootstrap",
                    AWS_LS_IO_FILE_UTILS => "file_utils",
                    AWS_LS_IO_SHARED_LIBRARY => "shared_library",
                    AWS_LS_IO_EXPONENTIAL_BACKOFF_RETRY_STRATEGY => "exponential_backoff_retry_strategy",
                    AWS_LS_IO_STANDARD_RETRY_STRATEGY => "standard_retry_strategy",
                    AWS_LS_IO_PKCS11 => "pks11",
                    AWS_LS_IO_PEM => "pem",
                }
                HTTP as "http" {
                    AWS_LS_HTTP_CONNECTION => "connection",
                    AWS_LS_HTTP_ENCODER => "encoder",
                    AWS_LS_HTTP_DECODER => "decoder",
                    AWS_LS_HTTP_SERVER => "server",
                    AWS_LS_HTTP_STREAM => "stream",
                    AWS_LS_HTTP_CONNECTION_MANAGER => "connection_manager",
                    AWS_LS_HTTP_STREAM_MANAGER => "stream_manager",
                    AWS_LS_HTTP_WEBSOCKET => "websocket",
                    AWS_LS_HTTP_WEBSOCKET_SETUP => "websocket_setup",
                    AWS_LS_HTTP_PROXY_NEGOTIATION => "proxy_negotiation",
                }
                COMPRESSION as "compression" {}
                EVENT_STREAM as "event_stream" {
                    AWS_LS_EVENT_STREAM_CHANNEL_HANDLER => "channel_handler",
                    AWS_LS_EVENT_STREAM_RPC_SERVER => "rpc_server",
                    AWS_LS_EVENT_STREAM_RPC_CLIENT => "rpc_client",
                }
                MQTT as "mqtt" {
                    AWS_LS_MQTT_CLIENT => "client",
                    AWS_LS_MQTT_TOPIC_TREE => "topic_tree",
                    AWS_LS_MQTT5_GENERAL => "v5::general",
                    AWS_LS_MQTT5_CLIENT => "v5::client",
                    AWS_LS_MQTT5_CANARY => "v5::canary",
                    AWS_LS_MQTT5_TO_MQTT3_ADAPTER => "v5::mqtt3_adapter",
                }
                AUTH as "auth" {
                    AWS_LS_AUTH_PROFILE => "profile",
                    AWS_LS_AUTH_CREDENTIALS_PROVIDER => "credentials_provider",
                    AWS_LS_AUTH_SIGNING => "signing",
                    AWS_LS_IMDS_CLIENT => "imds",
                }
                CAL as "cal" {
                    AWS_LS_CAL_ECC => "ecc",
                    AWS_LS_CAL_HASH => "hash",
                    AWS_LS_CAL_HMAC => "hmac",
                    AWS_LS_CAL_DER => "der",
                    AWS_LS_CAL_LIBCRYPTO_RESOLVE => "libcrypto_resolve",
                    AWS_LS_CAL_RSA => "rsa",
                }
                IOTDEVICE as "iotdevice" {
                    AWS_LS_IOTDEVICE_DEFENDER_TASK => "defender_task",
                    AWS_LS_IOTDEVICE_DEFENDER_TASK_CONFIG => "defender_task_config",
                    AWS_LS_IOTDEVICE_NETWORK_CONFIG => "network_config",
                    AWS_LS_IOTDEVICE_SECURE_TUNNELING => "secure_tunneling",
                }
                S3 as "s3" {
                    AWS_LS_S3_CLIENT => "client",
                    AWS_LS_S3_CLIENT_STATS => "client_stats",
                    AWS_LS_S3_REQUEST => "requests",
                    AWS_LS_S3_META_REQUEST => "meta_requests",
                    AWS_LS_S3_ENDPOINT => "endpoint",
                }
                SDKUTILS as "sdkutils" {
                    AWS_LS_SDKUTILS_PROFILE => "profile",
                    AWS_LS_SDKUTILS_ENDPOINTS_PARSING => "endpoints_parsing",
                    AWS_LS_SDKUTILS_ENDPOINTS_RESOLVE => "endpoints_resolve",
                    AWS_LS_SDKUTILS_ENDPOINTS_GENERAL => "endpoints_general",
                    AWS_LS_SDKUTILS_PARTITIONS_PARSING => "partitions_parsing",
                }
            }
        )
    }
}
