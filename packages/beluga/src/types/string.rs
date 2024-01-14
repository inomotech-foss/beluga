use core::fmt::{Debug, Display};
use core::ops::{Deref, DerefMut};
use core::ptr::{addr_of_mut, NonNull};

use aws_c_common_sys::{aws_mem_acquire, aws_mem_realloc, aws_string, aws_string_destroy};

use crate::{Allocator, AllocatorRef, Error, Result};

#[repr(transparent)]
pub struct AwsStr(aws_string);

impl AwsStr {
    #[inline]
    fn allocator(&self) -> Option<AllocatorRef> {
        if self.0.allocator.is_null() {
            None
        } else {
            Some(unsafe { Allocator::new(self.0.allocator) })
        }
    }

    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut aws_string {
        &mut self.0
    }

    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len
    }

    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    #[must_use]
    pub const fn as_bytes_ptr(&self) -> *const u8 {
        self.0.bytes.as_ptr()
    }

    #[inline]
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.as_bytes_ptr(), self.len()) }
    }
}

impl AsRef<[u8]> for AwsStr {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Debug for AwsStr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "\"{}\"", self.as_bytes().escape_ascii())
    }
}

impl Display for AwsStr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.as_bytes().escape_ascii())
    }
}

#[repr(transparent)]
pub struct AwsString(NonNull<AwsStr>);

impl AwsString {
    #[inline]
    const fn as_inner_ptr(&self) -> *mut aws_string {
        self.0.as_ptr().cast::<aws_string>()
    }

    #[inline]
    fn as_str(&self) -> &AwsStr {
        unsafe { self.0.as_ref() }
    }

    #[inline]
    fn as_str_mut(&mut self) -> &mut AwsStr {
        unsafe { self.0.as_mut() }
    }
}

impl Drop for AwsString {
    fn drop(&mut self) {
        unsafe { aws_string_destroy(self.as_mut_ptr()) };
    }
}

impl Debug for AwsString {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(self.as_str(), f)
    }
}

impl Display for AwsString {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Display::fmt(self.as_str(), f)
    }
}

impl Deref for AwsString {
    type Target = AwsStr;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl DerefMut for AwsString {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_str_mut()
    }
}

impl AwsString {
    #[inline]
    pub fn new(allocator: AllocatorRef) -> Result<Self> {
        // SAFETY: this is safe because the length is 0 so there's no uninitialized
        // memory
        unsafe { Self::allocate_uninit(allocator, 0) }
    }

    #[inline]
    fn get_allocation_size(len: usize) -> Result<usize> {
        len.checked_add(core::mem::size_of::<aws_string>())
            .ok_or(Error::OVERFLOW_DETECTED)
    }

    unsafe fn allocate_uninit(allocator: AllocatorRef, len: usize) -> Result<Self> {
        let ptr = aws_mem_acquire(allocator.as_ptr(), Self::get_allocation_size(len)?)
            .cast::<aws_string>();
        if ptr.is_null() {
            return Err(Error::last_in_current_thread());
        }
        addr_of_mut!((*ptr).allocator).write(allocator.as_ptr());
        addr_of_mut!((*ptr).len).write(len);
        addr_of_mut!((*ptr).bytes).cast::<u8>().add(len).write(0);
        Ok(Self(NonNull::new_unchecked(ptr).cast()))
    }

    /// # Safety
    ///
    /// The allocator must be the same allocator as the string is using.
    unsafe fn realloc_uninit(&mut self, allocator: AllocatorRef, len: usize) -> Result<()> {
        let old_size = Self::get_allocation_size(self.len())?;
        let new_size = Self::get_allocation_size(len)?;

        Error::check_rc(unsafe {
            aws_mem_realloc(
                allocator.as_ptr(),
                core::ptr::addr_of_mut!(self.0).cast(),
                old_size,
                new_size,
            )
        })?;
        let ptr = self.as_inner_ptr();
        addr_of_mut!((*ptr).len).write(len);
        addr_of_mut!((*ptr).bytes).cast::<u8>().add(len).write(0);

        Ok(())
    }

    pub fn try_push_str(&mut self, s: &str) -> Result<()> {
        let allocator = self.allocator().ok_or(Error::UNKNOWN)?;
        let old_len = self.len();
        unsafe {
            self.realloc_uninit(allocator, old_len + s.len())?;
            core::ptr::copy_nonoverlapping(
                s.as_ptr(),
                self.as_bytes_ptr().cast_mut().add(old_len),
                s.len(),
            );
        }
        Ok(())
    }
}

impl core::fmt::Write for AwsString {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.try_push_str(s).map_err(|_| core::fmt::Error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drop_empty() {
        let buf = AwsString::new(Allocator::default()).unwrap();
        drop(buf);
    }

    #[test]
    fn push_str() {
        let mut buf = AwsString::new(Allocator::default()).unwrap();
        assert_eq!(buf.as_bytes(), []);

        buf.try_push_str("hello ").unwrap();
        assert_eq!(buf.as_bytes(), b"hello ");

        buf.try_push_str("world").unwrap();
        assert_eq!(buf.as_bytes(), b"hello world");
    }
}
