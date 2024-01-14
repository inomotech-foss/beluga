use core::ffi::c_void;

use aws_c_common_sys::{aws_allocator, aws_default_allocator};

pub type AllocatorRef = &'static Allocator;

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
