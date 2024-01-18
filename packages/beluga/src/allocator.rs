use core::alloc::Layout;
use core::ffi::c_void;
use core::fmt::Debug;
use core::mem::{align_of, size_of};

use aws_c_common_sys::{aws_allocator, aws_default_allocator};

pub type AllocatorRef = &'static Allocator;

impl Default for AllocatorRef {
    fn default() -> Self {
        Allocator::rust()
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
    pub fn aws_default() -> AllocatorRef {
        unsafe { Self::new(aws_default_allocator()) }
    }

    #[inline]
    #[must_use]
    pub const fn as_ptr(&self) -> *mut aws_allocator {
        core::ptr::addr_of!(self.0).cast_mut()
    }

    #[inline]
    #[must_use]
    pub fn rust() -> AllocatorRef {
        static GLOBAL_ALLOCATOR: Allocator = Allocator(aws_allocator {
            mem_acquire: Some(mem_acquire),
            mem_release: Some(mem_release),
            mem_realloc: Some(mem_realloc),
            mem_calloc: Some(mem_calloc),
            impl_: core::ptr::null_mut(),
        });
        &GLOBAL_ALLOCATOR
    }
}

unsafe impl Send for Allocator {}
unsafe impl Sync for Allocator {}

/// Allocates memory for the given size using the standard memory allocator.
///
/// # Arguments
/// - `size` - size of desired allocation. Assume that size is properly aligned.
///
/// # Safety
///
/// The caller must ensure the given size is valid. The returned pointer
/// must be properly deallocated later.
#[inline]
unsafe extern "C" fn mem_acquire(_: *mut aws_allocator, size: usize) -> *mut c_void {
    // Calculates the Layout to allocate for the block, including space for
    // storing the Layout at the start.
    let layout =
        Layout::from_size_align_unchecked(size + size_of::<Layout>(), align_of::<Layout>());
    // Allocates memory using the provided layout, writes the layout to the
    // start of the allocated block, and returns a pointer offset past the
    // written layout for use as the allocated memory region.
    let ptr = std::alloc::alloc(layout);

    if ptr.is_null() {
        return core::ptr::null_mut();
    }

    core::ptr::write(ptr.cast(), layout);
    ptr.cast::<Layout>().add(1).cast()
}

#[inline]
unsafe extern "C" fn mem_release(_: *mut aws_allocator, ptr: *mut c_void) {
    let layout_ptr = ptr.cast::<Layout>().offset(-1);
    let layout = core::ptr::read(layout_ptr);
    std::alloc::dealloc(layout_ptr.cast(), layout);
}

/// # Safety
///
/// `mem_realloc` is optional and might not be present.
#[inline]
unsafe extern "C" fn mem_realloc(
    _: *mut aws_allocator,
    old_ptr: *mut c_void,
    _old_size: usize,
    new_size: usize,
) -> *mut c_void {
    // Reads the layout information stored at the start of the old memory
    // block. Uses this layout to reallocate the block to the new
    // size. Writes the updated layout to the start of the newly
    // allocated block. Returns a pointer to the new block, offset
    // past the layout information.
    let layout_ptr = old_ptr.cast::<Layout>().offset(-1);
    let layout = core::ptr::read(layout_ptr);
    let new_ptr = std::alloc::realloc(layout_ptr.cast(), layout, new_size + size_of::<Layout>());

    // Writes the layout information to the start of the newly allocated
    // memory. This stores the size and alignment so the memory can
    // be properly freed later.
    std::ptr::write(
        new_ptr.cast(),
        Layout::from_size_align_unchecked(new_size + size_of::<Layout>(), align_of::<Layout>()),
    );

    new_ptr.cast::<Layout>().add(1).cast()
}

/// # Safety
///
/// `mem_calloc` is optional and might not be present.
#[inline]
unsafe extern "C" fn mem_calloc(_: *mut aws_allocator, num: usize, size: usize) -> *mut c_void {
    let layout =
        Layout::from_size_align_unchecked(size * num + size_of::<Layout>(), align_of::<Layout>());
    let ptr = std::alloc::alloc_zeroed(layout);

    if ptr.is_null() {
        return core::ptr::null_mut();
    }

    core::ptr::write(ptr.cast(), layout);
    ptr.cast::<Layout>().add(1).cast()
}

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
