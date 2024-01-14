use aws_c_io_sys::{
    aws_socket_domain, aws_socket_options, aws_socket_type, AWS_SOCKET_DGRAM, AWS_SOCKET_IPV4,
    AWS_SOCKET_IPV6, AWS_SOCKET_LOCAL, AWS_SOCKET_STREAM, AWS_SOCKET_VSOCK,
};

#[repr(transparent)]
pub struct SocketOptions(aws_socket_options);

impl SocketOptions {
    #[must_use]
    pub const fn builder() -> SocketOptionsBuilder {
        SocketOptionsBuilder::new()
    }

    #[must_use]
    pub const fn as_ptr(&self) -> *mut aws_socket_options {
        core::ptr::addr_of!(self.0).cast_mut()
    }
}

pub struct SocketOptionsBuilder {
    inner: aws_socket_options,
}

impl SocketOptionsBuilder {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            inner: aws_socket_options {
                type_: AWS_SOCKET_STREAM,
                domain: AWS_SOCKET_IPV4,
                connect_timeout_ms: u32::MAX,
                keep_alive_interval_sec: 0,
                keep_alive_timeout_sec: 0,
                keep_alive_max_failed_probes: 0,
                keepalive: false,
            },
        }
    }

    pub fn build(&mut self) -> SocketOptions {
        SocketOptions(self.inner)
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct SocketType(aws_socket_type);

impl SocketType {
    pub const STREAM: Self = Self(AWS_SOCKET_STREAM);
    pub const DGRAM: Self = Self(AWS_SOCKET_DGRAM);
}

#[derive(Debug)]
#[repr(transparent)]
pub struct SocketDomain(aws_socket_domain);

impl SocketDomain {
    pub const IPV4: Self = Self(AWS_SOCKET_IPV4);
    pub const IPV6: Self = Self(AWS_SOCKET_IPV6);
    pub const LOCAL: Self = Self(AWS_SOCKET_LOCAL);
    pub const VSOCK: Self = Self(AWS_SOCKET_VSOCK);
}
