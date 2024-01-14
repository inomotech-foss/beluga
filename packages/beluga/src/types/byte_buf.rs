use core::fmt::Debug;
use core::marker::PhantomData;
use core::mem::MaybeUninit;

use aws_c_common_sys::{aws_byte_buf, aws_byte_buf_clean_up, aws_byte_buf_init, aws_byte_cursor};

use crate::allocator::AllocatorRef;
use crate::{Error, Result};

#[repr(transparent)]
pub struct ByteCursor<'a> {
    inner: aws_byte_cursor,
    marker: PhantomData<&'a [u8]>,
}

impl<'a> ByteCursor<'a> {
    /// # Safety
    ///
    /// The byte cursor lifetime must be valid for the lifetime of 'self'.
    #[inline]
    #[must_use]
    pub const unsafe fn from_inner(inner: aws_byte_cursor) -> Self {
        Self {
            inner,
            marker: PhantomData,
        }
    }

    #[inline]
    #[must_use]
    pub const fn empty() -> Self {
        unsafe {
            Self::from_inner(aws_byte_cursor {
                len: 0,
                ptr: core::ptr::null_mut(),
            })
        }
    }

    #[inline]
    #[must_use]
    pub const unsafe fn from_ptr(cursor: *const aws_byte_cursor) -> Self {
        Self::from_inner(*cursor)
    }

    #[inline]
    #[must_use]
    pub const fn as_ptr(&self) -> *const aws_byte_cursor {
        &self.inner
    }

    #[inline]
    #[must_use]
    pub const fn into_inner(self) -> aws_byte_cursor {
        self.inner
    }

    #[inline]
    #[must_use]
    pub const fn from_slice(b: &'a [u8]) -> Self {
        // SAFETY: the lifetime of the slice applies to Self
        unsafe {
            // not using 'aws_byte_cursor_from_array' so the function can be const
            Self::from_inner(aws_byte_cursor {
                len: b.len(),
                ptr: b.as_ptr().cast_mut(),
            })
        }
    }

    #[inline]
    #[must_use]
    pub const fn from_str(s: &'a str) -> Self {
        Self::from_slice(s.as_bytes())
    }

    #[inline]
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.inner.ptr, self.inner.len) }
    }
}

impl<'a> Debug for ByteCursor<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("ByteCursor").field(&self.as_bytes()).finish()
    }
}

#[repr(transparent)]
pub struct ByteBuf(aws_byte_buf);

impl ByteBuf {
    pub fn with_capacity(allocator: AllocatorRef, capacity: usize) -> Result<Self> {
        let mut buf = MaybeUninit::uninit();
        Error::check_rc(unsafe {
            aws_byte_buf_init(buf.as_mut_ptr(), allocator.as_ptr(), capacity)
        })?;
        Ok(Self(unsafe { buf.assume_init() }))
    }

    pub fn as_mut_ptr(&mut self) -> *mut aws_byte_buf {
        &mut self.0
    }

    #[inline]
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.0.buffer, self.0.len) }
    }

    #[inline]
    #[must_use]
    pub const fn cursor(&self) -> ByteCursor {
        ByteCursor::from_slice(self.as_bytes())
    }
}

impl Drop for ByteBuf {
    fn drop(&mut self) {
        unsafe { aws_byte_buf_clean_up(self.as_mut_ptr()) };
    }
}
