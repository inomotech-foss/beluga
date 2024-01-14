use alloc::sync::Arc;
use core::cell::UnsafeCell;
use core::ffi::{c_int, c_void};
use core::fmt::Debug;
use core::future::Future;
use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering::SeqCst;
use core::task::{Poll, Waker};

pub fn create<T>() -> (CallbackFutureResolver<T>, CallbackFuture<T>) {
    let inner = Arc::new(Inner::new());
    (
        CallbackFutureResolver {
            inner: inner.clone(),
        },
        CallbackFuture { inner },
    )
}

pub struct CallbackFutureResolver<T> {
    inner: Arc<Inner<T>>,
}

impl<T> CallbackFutureResolver<T> {
    pub fn into_raw(self) -> *mut c_void {
        let ptr = Arc::as_ptr(&self.inner).cast::<c_void>().cast_mut();
        core::mem::forget(self);
        ptr
    }

    pub unsafe fn from_raw(raw: *mut c_void) -> Self {
        Self {
            inner: Arc::from_raw(raw.cast()),
        }
    }

    pub fn resolve(self, value: T) {
        let _ = self.inner.send(value);
    }
}

impl<T> CallbackFutureResolver<crate::Result<T>> {
    pub fn try_or_resolve(self, f: impl FnOnce(*mut c_void) -> c_int) {
        let userdata = self.into_raw();
        if let Err(err) = crate::Error::check_rc(f(userdata)) {
            // SAFETY: since the operation failed, the resolver hasn't been dropped yet
            let resolver = unsafe { Self::from_raw(userdata) };
            resolver.resolve(Err(err));
        }
    }
}

impl<T> Drop for CallbackFutureResolver<T> {
    fn drop(&mut self) {
        self.inner.drop_tx();
    }
}
pub struct CallbackFuture<T> {
    inner: Arc<Inner<T>>,
}

impl<T> CallbackFuture<T> {
    pub fn try_poll(&self) -> Option<T> {
        self.inner.try_recv()
    }
}

// we never project to the inner T
impl<T> Unpin for CallbackFuture<T> {}

impl<T> Future for CallbackFuture<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> Poll<T> {
        self.inner.recv(cx)
    }
}

impl<T> Drop for CallbackFuture<T> {
    fn drop(&mut self) {
        self.inner.drop_rx();
    }
}

impl<T> Debug for CallbackFuture<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CallbackFuture")
            .field("complete", &self.inner.complete)
            .finish()
    }
}

// the following types are copied from futures' oneshot implementation: <https://github.com/rust-lang/futures-rs/blob/b2f9298f31731260a09e9ad62a3df3456bc3004e/futures-channel/src/oneshot.rs#L36>
// we can't use the oneshot implementation directly because it lacks the
// into_raw / from_raw methods. Perhaps they might be willing to accept a PR for
// this?

struct Inner<T> {
    complete: AtomicBool,
    data: Lock<Option<T>>,
    rx_task: Lock<Option<Waker>>,
}

impl<T> Inner<T> {
    fn new() -> Self {
        Self {
            complete: AtomicBool::new(false),
            data: Lock::new(None),
            rx_task: Lock::new(None),
        }
    }

    fn send(&self, t: T) -> Result<(), T> {
        if self.complete.load(SeqCst) {
            return Err(t);
        }

        // Note that this lock acquisition may fail if the receiver
        // is closed and sets the `complete` flag to `true`, whereupon
        // the receiver may call `poll()`.
        if let Some(mut slot) = self.data.try_lock() {
            assert!(slot.is_none());
            *slot = Some(t);
            drop(slot);

            // If the receiver called `close()` between the check at the
            // start of the function, and the lock being released, then
            // the receiver may not be around to receive it, so try to
            // pull it back out.
            if self.complete.load(SeqCst) {
                // If lock acquisition fails, then receiver is actually
                // receiving it, so we're good.
                if let Some(mut slot) = self.data.try_lock() {
                    if let Some(t) = slot.take() {
                        return Err(t);
                    }
                }
            }
            Ok(())
        } else {
            // Must have been closed
            Err(t)
        }
    }

    fn is_canceled(&self) -> bool {
        self.complete.load(SeqCst)
    }

    fn drop_tx(&self) {
        // Flag that we're a completed `Sender` and try to wake up a receiver.
        // Whether or not we actually stored any data will get picked up and
        // translated to either an item or cancellation.
        //
        // Note that if we fail to acquire the `rx_task` lock then that means
        // we're in one of two situations:
        //
        // 1. The receiver is trying to block in `poll`
        // 2. The receiver is being dropped
        //
        // In the first case it'll check the `complete` flag after it's done
        // blocking to see if it succeeded. In the latter case we don't need to
        // wake up anyone anyway. So in both cases it's ok to ignore the `None`
        // case of `try_lock` and bail out.
        //
        // The first case crucially depends on `Lock` using `SeqCst` ordering
        // under the hood. If it instead used `Release` / `Acquire` ordering,
        // then it would not necessarily synchronize with `inner.complete`
        // and deadlock might be possible, as was observed in
        // https://github.com/rust-lang/futures-rs/pull/219.
        self.complete.store(true, SeqCst);

        if let Some(mut slot) = self.rx_task.try_lock() {
            if let Some(task) = slot.take() {
                drop(slot);
                task.wake();
            }
        }
    }

    fn try_recv(&self) -> Option<T> {
        // If we're complete, either `::close_rx` or `::drop_tx` was called.
        // We can assume a successful send if data is present.
        if self.complete.load(SeqCst) {
            if let Some(mut slot) = self.data.try_lock() {
                if let Some(data) = slot.take() {
                    return Some(data);
                }
            }
        }
        None
    }

    fn recv(&self, cx: &mut core::task::Context<'_>) -> Poll<T> {
        // Check to see if some data has arrived. If it hasn't then we need to
        // block our task.
        //
        // Note that the acquisition of the `rx_task` lock might fail below, but
        // the only situation where this can happen is during `Sender::drop`
        // when we are indeed completed already. If that's happening then we
        // know we're completed so keep going.
        let done = if self.complete.load(SeqCst) {
            true
        } else {
            let task = cx.waker().clone();
            match self.rx_task.try_lock() {
                Some(mut slot) => {
                    *slot = Some(task);
                    false
                }
                None => true,
            }
        };

        // If we're `done` via one of the paths above, then look at the data and
        // figure out what the answer is. If, however, we stored `rx_task`
        // successfully above we need to check again if we're completed in case
        // a message was sent while `rx_task` was locked and couldn't notify us
        // otherwise.
        //
        // If we're not done, and we're not complete, though, then we've
        // successfully blocked our task and we return `Pending`.
        if done || self.complete.load(SeqCst) {
            // If taking the lock fails, the sender will realise that the we're
            // `done` when it checks the `complete` flag on the way out, and
            // will treat the send as a failure.
            if let Some(mut slot) = self.data.try_lock() {
                if let Some(data) = slot.take() {
                    return Poll::Ready(data);
                }
            }
        }
        Poll::Pending
    }

    fn drop_rx(&self) {
        // Indicate to the `Sender` that we're done, so any future calls to
        // `poll_canceled` are weeded out.
        self.complete.store(true, SeqCst);

        // If we've blocked a task then there's no need for it to stick around,
        // so we need to drop it. If this lock acquisition fails, though, then
        // it's just because our `Sender` is trying to take the task, so we
        // let them take care of that.
        if let Some(mut slot) = self.rx_task.try_lock() {
            let task = slot.take();
            drop(slot);
            drop(task);
        }
    }
}

/// A "mutex" around a value, similar to `core::sync::Mutex<T>`.
///
/// This lock only supports the `try_lock` operation, however, and does not
/// implement poisoning.
#[derive(Debug)]
pub struct Lock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

/// Sentinel representing an acquired lock through which the data can be
/// accessed.
pub struct TryLock<'a, T> {
    __ptr: &'a Lock<T>,
}

// The `Lock` structure is basically just a `Mutex<T>`, and these two impls are
// intended to mirror the standard library's corresponding impls for `Mutex<T>`.
//
// If a `T` is sendable across threads, so is the lock, and `T` must be sendable
// across threads to be `Sync` because it allows mutable access from multiple
// threads.
unsafe impl<T: Send> Send for Lock<T> {}
unsafe impl<T: Send> Sync for Lock<T> {}

impl<T> Lock<T> {
    /// Creates a new lock around the given value.
    pub(crate) fn new(t: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(t),
        }
    }

    /// Attempts to acquire this lock, returning whether the lock was acquired
    /// or not.
    ///
    /// If `Some` is returned then the data this lock protects can be accessed
    /// through the sentinel. This sentinel allows both mutable and immutable
    /// access.
    ///
    /// If `None` is returned then the lock is already locked, either elsewhere
    /// on this thread or on another thread.
    pub(crate) fn try_lock(&self) -> Option<TryLock<'_, T>> {
        if !self.locked.swap(true, SeqCst) {
            Some(TryLock { __ptr: self })
        } else {
            None
        }
    }
}

impl<T> Deref for TryLock<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // The existence of `TryLock` represents that we own the lock, so we
        // can safely access the data here.
        unsafe { &*self.__ptr.data.get() }
    }
}

impl<T> DerefMut for TryLock<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // The existence of `TryLock` represents that we own the lock, so we
        // can safely access the data here.
        //
        // Additionally, we're the *only* `TryLock` in existence so mutable
        // access should be ok.
        unsafe { &mut *self.__ptr.data.get() }
    }
}

impl<T> Drop for TryLock<'_, T> {
    fn drop(&mut self) {
        self.__ptr.locked.store(false, SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::Lock;

    #[test]
    fn smoke() {
        let a = Lock::new(1);
        let mut a1 = a.try_lock().unwrap();
        assert!(a.try_lock().is_none());
        assert_eq!(*a1, 1);
        *a1 = 2;
        drop(a1);
        assert_eq!(*a.try_lock().unwrap(), 2);
        assert_eq!(*a.try_lock().unwrap(), 2);
    }
}
