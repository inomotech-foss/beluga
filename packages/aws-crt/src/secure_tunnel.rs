use core::ffi::CStr;
use core::marker::PhantomData;

use aws_c_iot_sys::{
    aws_secure_tunnel, aws_secure_tunnel_acquire, aws_secure_tunnel_connection_start,
    aws_secure_tunnel_message_view, aws_secure_tunnel_new, aws_secure_tunnel_options,
    aws_secure_tunnel_release, aws_secure_tunnel_send_message, aws_secure_tunnel_start,
    aws_secure_tunnel_stop, aws_secure_tunnel_stream_start, AWS_SECURE_TUNNELING_DESTINATION_MODE,
    AWS_SECURE_TUNNELING_SOURCE_MODE,
};

use crate::{Allocator, AllocatorRef, ByteCursor, Error, Result};

mod callbacks;

ref_counted_wrapper!(struct Inner(aws_secure_tunnel) {
    acquire: aws_secure_tunnel_acquire,
    release: aws_secure_tunnel_release,
});

#[derive(Clone)]
pub struct Tunnel(Inner);

impl Tunnel {
    #[must_use]
    pub const fn builder() -> Builder<'static> {
        Builder::new()
    }

    /// Creates a new secure tunnel
    fn new(allocator: AllocatorRef, options: &aws_secure_tunnel_options) -> Result<Self> {
        unsafe { Inner::new_or_error(aws_secure_tunnel_new(allocator.as_ptr(), options)) }.map(Self)
    }

    /// Asynchronous notify to the secure tunnel that you want it to attempt to
    /// connect.
    ///
    /// The secure tunnel will attempt to stay connected.
    pub fn start(&self) -> Result<()> {
        Error::check_rc(unsafe { aws_secure_tunnel_start(self.0.as_ptr()) })
    }

    /// Asynchronous notify to the secure tunnel that you want it to transition
    /// to the stopped state.
    ///
    /// When the secure tunnel reaches the stopped
    /// state, all session state is erased.
    pub fn stop(&self) -> Result<()> {
        Error::check_rc(unsafe { aws_secure_tunnel_stop(self.0.as_ptr()) })
    }

    /// Queues a message operation in a secure tunnel
    fn send_message(&self, options: &aws_secure_tunnel_message_view) -> Result<()> {
        Error::check_rc(unsafe { aws_secure_tunnel_send_message(self.0.as_ptr(), options) })
    }

    /// Queue a `STREAM_START` message in a secure tunnel
    ///
    /// THIS API SHOULD ONLY BE USED FROM SOURCE MODE
    fn stream_start(&self, options: &aws_secure_tunnel_message_view) -> Result<()> {
        Error::check_rc(unsafe { aws_secure_tunnel_stream_start(self.0.as_ptr(), options) })
    }

    /// Queue a `CONNECTION_START` message in a secure tunnel
    ///
    /// THIS API SHOULD ONLY BE USED FROM SOURCE MODE
    fn connection_start(&self, options: &aws_secure_tunnel_message_view) -> Result<()> {
        Error::check_rc(unsafe { aws_secure_tunnel_connection_start(self.0.as_ptr(), options) })
    }
}

pub struct Builder<'a> {
    allocator: Option<AllocatorRef>,
    options: aws_secure_tunnel_options,
    marker: PhantomData<&'a ()>,
}

impl Builder<'static> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            allocator: None,
            options: aws_secure_tunnel_options {
                endpoint_host: ByteCursor::empty().into_inner(),
                bootstrap: core::ptr::null_mut(),
                socket_options: core::ptr::null_mut(),
                tls_options: core::ptr::null_mut(),
                http_proxy_options: core::ptr::null(),
                access_token: ByteCursor::empty().into_inner(),
                client_token: ByteCursor::empty().into_inner(),
                root_ca: core::ptr::null(),
                on_message_received: None,
                user_data: core::ptr::null_mut(),
                local_proxy_mode: 0,
                on_connection_complete: None,
                on_connection_shutdown: None,
                on_send_message_complete: None,
                on_stream_start: None,
                on_stream_reset: None,
                on_connection_start: None,
                on_connection_reset: None,
                on_session_reset: None,
                on_stopped: None,
                on_termination_complete: None,
                secure_tunnel_on_termination_user_data: core::ptr::null_mut(),
            },
            marker: PhantomData,
        }
    }
}

impl Default for Builder<'static> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Builder<'a> {
    pub fn build(&mut self) -> Result<Tunnel> {
        let allocator = self.allocator.unwrap_or(Allocator::default());
        Tunnel::new(allocator, &self.options)
    }

    pub fn allocator(&mut self, allocator: AllocatorRef) -> &mut Self {
        self.allocator = Some(allocator);
        self
    }

    pub fn source_mode(&mut self) -> &mut Self {
        self.options.local_proxy_mode = AWS_SECURE_TUNNELING_SOURCE_MODE;
        self
    }

    pub fn destination_mode(&mut self) -> &mut Self {
        self.options.local_proxy_mode = AWS_SECURE_TUNNELING_DESTINATION_MODE;
        self
    }

    /// Host to establish Secure Tunnel connection to
    pub fn endpoint_host(&mut self, value: &'a str) -> &mut Self {
        self.options.endpoint_host = ByteCursor::from_str(value).into_inner();
        self
    }

    /// Access Token used to establish a Secure Tunnel connection
    pub fn access_token(&mut self, value: &'a str) -> &mut Self {
        self.options.access_token = ByteCursor::from_str(value).into_inner();
        self
    }

    /// (Optional) Client Token used to re-establish a Secure Tunnel connection
    /// after the one-time use access token has been used.
    ///
    /// If one is not provided, it will automatically be generated and re-used
    /// on subsequent reconnects.
    pub fn client_token(&mut self, value: &'a str) -> &mut Self {
        self.options.client_token = ByteCursor::from_str(value).into_inner();
        self
    }

    pub fn root_ca(&mut self, value: &'a CStr) -> &mut Self {
        self.options.root_ca = value.as_ptr();
        self
    }
}
