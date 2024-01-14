use core::marker::PhantomData;

use aws_c_io_sys::{
    aws_host_resolver, aws_host_resolver_acquire, aws_host_resolver_default_options,
    aws_host_resolver_new_default, aws_host_resolver_release,
};

use super::EventLoopGroup;
use crate::{Allocator, AllocatorRef, Result};

ref_counted_wrapper!(struct Inner(aws_host_resolver) {
    acquire: aws_host_resolver_acquire,
    release: aws_host_resolver_release,
});

#[derive(Clone)]
pub struct HostResolver(Inner);

impl HostResolver {
    #[must_use]
    pub const fn builder(el_group: &EventLoopGroup) -> HostResolverBuilder<'_> {
        HostResolverBuilder::new(el_group)
    }

    fn new_default(
        allocator: AllocatorRef,
        options: &aws_host_resolver_default_options,
    ) -> Result<Self> {
        unsafe { Inner::new_or_error(aws_host_resolver_new_default(allocator.as_ptr(), options)) }
            .map(Self)
    }

    #[must_use]
    pub const fn as_ptr(&self) -> *mut aws_host_resolver {
        self.0.as_ptr()
    }

    pub async fn wait_shutdown(self) {
        drop(self); // release my handle
        todo!()
    }
}

pub struct HostResolverBuilder<'a> {
    allocator: Option<AllocatorRef>,
    options: aws_host_resolver_default_options,
    marker: PhantomData<&'a ()>,
}

impl<'a> HostResolverBuilder<'a> {
    #[must_use]
    pub const fn new(el_group: &'a EventLoopGroup) -> Self {
        Self {
            allocator: None,
            options: aws_host_resolver_default_options {
                max_entries: 0,
                el_group: el_group.as_ptr(),
                shutdown_options: core::ptr::null(),
                system_clock_override_fn: None,
            },
            marker: PhantomData,
        }
    }

    pub fn build(&mut self) -> Result<HostResolver> {
        let allocator = self.allocator.unwrap_or(Allocator::default());
        HostResolver::new_default(allocator, &self.options)
    }

    pub fn allocator(&mut self, allocator: AllocatorRef) -> &mut Self {
        self.allocator = Some(allocator);
        self
    }

    pub fn max_entries(&mut self, value: usize) -> &mut Self {
        self.options.max_entries = value;
        self
    }
}
