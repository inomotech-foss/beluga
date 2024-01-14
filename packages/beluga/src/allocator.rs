use core::ffi::c_void;
use core::fmt::Debug;

use aws_c_common_sys::{aws_allocator, aws_default_allocator};

pub type AllocatorRef = &'static Allocator;

impl Default for AllocatorRef {
    fn default() -> Self {
        Allocator::default()
    }
}

#[repr(transparent)]
pub struct Allocator(aws_allocator);

impl Allocator {
    /// # Safety
    ///
    /// `allocator` must be a valid (non-null) pointer with static lifetime.
    #[inline]
    pub(crate) unsafe fn new(allocator: *mut aws_allocator) -> AllocatorRef {
        debug_assert!(!allocator.is_null());
        &mut *allocator.cast()
    }

    #[inline]
    #[must_use]
    pub fn default() -> AllocatorRef {
        unsafe { Self::new(aws_default_allocator()) }
    }

    #[inline]
    #[must_use]
    pub const fn as_ptr(&self) -> *mut aws_allocator {
        core::ptr::addr_of!(self.0).cast_mut()
    }

    #[inline]
    unsafe fn mem_acquire(&self, size: usize) -> *mut c_void {
        let f = self.0.mem_acquire.unwrap_unchecked();
        f(self.as_ptr(), size)
    }

    #[inline]
    unsafe fn mem_release(&self, ptr: *mut c_void) {
        let f = self.0.mem_release.unwrap_unchecked();
        f(self.as_ptr(), ptr);
    }

    /// # Safety
    ///
    /// `mem_realloc` is optional and might not be present.
    #[inline]
    unsafe fn mem_realloc(
        &self,
        old_ptr: *mut c_void,
        old_size: usize,
        new_size: usize,
    ) -> *mut c_void {
        let f = self.0.mem_realloc.unwrap_unchecked();
        f(self.as_ptr(), old_ptr, old_size, new_size)
    }

    /// # Safety
    ///
    /// `mem_calloc` is optional and might not be present.
    #[inline]
    unsafe fn mem_calloc(&self, num: usize, size: usize) -> *mut c_void {
        let f = self.0.mem_calloc.unwrap_unchecked();
        f(self.as_ptr(), num, size)
    }
}

unsafe impl Send for Allocator {}
unsafe impl Sync for Allocator {}

impl Debug for Allocator {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // use the 'mem_acquire' function (which should always be there) as a kind of
        // "id" of the allocator.
        let ptr = self
            .0
            .mem_acquire
            .map_or(core::ptr::null(), |x| x as *const ());
        f.debug_tuple("Allocator").field(&ptr).finish()
    }
}

impl PartialEq for Allocator {
    fn eq(&self, other: &Self) -> bool {
        fn get_discriminant(alloc: &aws_allocator) -> impl PartialEq {
            (
                alloc.mem_acquire,
                alloc.mem_release,
                alloc.mem_realloc,
                alloc.mem_calloc,
                alloc.impl_,
            )
        }

        get_discriminant(&self.0) == get_discriminant(&other.0)
    }
}
