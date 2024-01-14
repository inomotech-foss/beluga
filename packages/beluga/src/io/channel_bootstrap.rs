use core::marker::PhantomData;

use aws_c_io_sys::{
    aws_client_bootstrap, aws_client_bootstrap_acquire, aws_client_bootstrap_new,
    aws_client_bootstrap_options, aws_client_bootstrap_release,
};

use super::{EventLoopGroup, HostResolver};
use crate::{Allocator, AllocatorRef, Result};

ref_counted_wrapper!(pub struct Inner(aws_client_bootstrap) {
    acquire: aws_client_bootstrap_acquire,
    release: aws_client_bootstrap_release,
});

/// ```rust
/// # use aws_crt::Allocator;
/// # use aws_crt::io::{EventLoopGroup, HostResolver, ClientBootstrap};
/// let el_group = EventLoopGroup::new_default(Allocator::default(), 1).unwrap();
/// let host_resolver = HostResolver::builder(&el_group).build().unwrap();
/// let client_bootstrap = ClientBootstrap::builder(&el_group, &host_resolver)
///     .build()
///     .unwrap();
/// ```
#[derive(Clone)]
pub struct ClientBootstrap(Inner);

impl ClientBootstrap {
    #[must_use]
    pub const fn builder<'a>(
        el_group: &'a EventLoopGroup,
        host_resolver: &'a HostResolver,
    ) -> ClientBootstrapBuilder<'a> {
        ClientBootstrapBuilder::new(el_group, host_resolver)
    }

    fn new(allocator: AllocatorRef, options: &aws_client_bootstrap_options) -> Result<Self> {
        unsafe { Inner::new_or_error(aws_client_bootstrap_new(allocator.as_ptr(), options)) }
            .map(Self)
    }

    #[must_use]
    pub const fn as_ptr(&self) -> *mut aws_client_bootstrap {
        self.0.as_ptr()
    }

    pub async fn wait_shutdown(self) {
        drop(self); // release my handle
        todo!()
    }
}

pub struct ClientBootstrapBuilder<'a> {
    allocator: Option<AllocatorRef>,
    options: aws_client_bootstrap_options,
    marker: PhantomData<&'a ()>,
}

impl<'a> ClientBootstrapBuilder<'a> {
    #[must_use]
    pub const fn new(el_group: &'a EventLoopGroup, host_resolver: &'a HostResolver) -> Self {
        Self {
            allocator: None,
            options: aws_client_bootstrap_options {
                event_loop_group: el_group.as_ptr(),
                host_resolver: host_resolver.as_ptr(),
                host_resolution_config: core::ptr::null(),
                on_shutdown_complete: None,
                user_data: core::ptr::null_mut(),
            },
            marker: PhantomData,
        }
    }

    pub fn build(&mut self) -> Result<ClientBootstrap> {
        let allocator = self.allocator.unwrap_or(Allocator::default());
        ClientBootstrap::new(allocator, &self.options)
    }

    pub fn allocator(&mut self, allocator: AllocatorRef) -> &mut Self {
        self.allocator = Some(allocator);
        self
    }
}
