use core::ffi::{c_int, c_void};
use core::future::Future;
use core::pin::Pin;
use core::task::Poll;

use futures::FutureExt;

use crate::future::{CallbackFuture, CallbackFutureResolver};
use crate::{Error, Result};

#[must_use]
#[derive(Debug)]
pub struct TaskFuture<T> {
    state: State<T>,
}

impl<T> TaskFuture<T> {
    pub const fn check(&self) -> Result<()> {
        self.state.check()
    }

    pub fn started(self) -> Result<Self> {
        self.check().map(|()| self)
    }

    pub(crate) fn create(res: Result<()>, fut: CallbackFuture<Result<T>>) -> Self {
        let state = match res {
            Ok(()) => State::Running(fut),
            Err(err) => State::Error(err),
        };
        Self { state }
    }

    pub(crate) unsafe fn resolve(userdata: *mut c_void, res: Result<T>) {
        let resolver = CallbackFutureResolver::<Result<T>>::from_raw(userdata);
        resolver.resolve(res);
    }
}

impl<T> Future for TaskFuture<T> {
    type Output = Result<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        this.state.poll_unpin(cx)
    }
}

#[must_use]
#[derive(Debug)]
pub struct PacketFuture<T> {
    packet_id: u16,
    state: State<T>,
}

impl<T> PacketFuture<T> {
    #[must_use]
    pub const fn packet_id(&self) -> u16 {
        self.packet_id
    }

    pub const fn check(&self) -> Result<()> {
        self.state.check()
    }

    pub fn started(self) -> Result<Self> {
        self.check().map(|()| self)
    }

    pub(crate) fn create(packet_id: u16, fut: CallbackFuture<Result<T>>) -> Self {
        let state = if packet_id == 0 {
            State::Error(Error::last_in_current_thread())
        } else {
            State::Running(fut)
        };
        Self { packet_id, state }
    }

    pub(crate) unsafe fn resolve(userdata: *mut c_void, res: Result<T>) {
        let resolver = CallbackFutureResolver::<Result<T>>::from_raw(userdata);
        resolver.resolve(res);
    }
}

impl PacketFuture<()> {
    pub(crate) unsafe fn resolve_with_error_code(userdata: *mut c_void, error_code: c_int) {
        Self::resolve(userdata, Error::check_rc(error_code));
    }
}

impl<T> Future for PacketFuture<T> {
    type Output = Result<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        this.state.poll_unpin(cx)
    }
}

#[derive(Debug)]
enum State<T> {
    Error(Error),
    Running(CallbackFuture<Result<T>>),
}

impl<T> State<T> {
    const fn check(&self) -> Result<()> {
        if let Self::Error(err) = *self {
            Err(err)
        } else {
            Ok(())
        }
    }
}

impl<T> Future for State<T> {
    type Output = Result<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        match this {
            Self::Error(err) => Poll::Ready(Err(*err)),
            Self::Running(fut) => fut.poll_unpin(cx),
        }
    }
}
