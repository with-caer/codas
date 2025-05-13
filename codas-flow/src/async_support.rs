//! Runtime-agnostic `async` utilities.

use core::{
    future::Future,
    pin::Pin,
    ptr::null,
    task::{Poll, RawWaker, RawWakerVTable, Waker},
};

/// Returns a future that becomes ready
/// after one poll, emulating a yield on
/// most async runtimes.
pub async fn yield_now() {
    YieldNow::Pending.await
}

/// Future returned by [`yield_now`].
enum YieldNow {
    /// The future has not yet yielded.
    Pending,

    /// The future has yielded for
    /// at least one poll cycle, and
    /// is now ready.
    Ready,
}

impl Future for YieldNow {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        match *self {
            YieldNow::Pending => {
                *self = YieldNow::Ready;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            YieldNow::Ready => Poll::Ready(()),
        }
    }
}

/// Returns an asynchronous [`Waker`] that
/// does nothing at all.
///
/// This implementation is based on the
/// [`futures` crate](https://github.com/rust-lang/futures-rs),
/// and may be removed in the future.
#[inline]
pub(crate) fn noop_waker() -> Waker {
    unsafe { Waker::from_raw(noop_raw_waker()) }
}

const NOOP_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(noop_clone, noop, noop, noop);

const unsafe fn noop_clone(_data: *const ()) -> RawWaker {
    noop_raw_waker()
}

const unsafe fn noop(_data: *const ()) {}

const fn noop_raw_waker() -> RawWaker {
    RawWaker::new(null(), &NOOP_WAKER_VTABLE)
}
