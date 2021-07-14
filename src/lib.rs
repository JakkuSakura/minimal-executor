mod local_pool;
pub(crate) mod waker;
pub use crate::local_pool::*;

use std::future::{Future};
use std::task::{Poll, Context};
use crate::waker::{AlwaysWake, waker_ref};

pub fn poll_fn<T, F: FnOnce(&mut Context<'_>) -> T>(f: F) -> T {
    let waker = waker_ref(&AlwaysWake::INSTANCE);
    let mut cx = Context::from_waker(&waker);
    f(&mut cx)
}

pub fn block_fn<T, F: FnMut(&mut Context<'_>) -> Poll<T>>(mut f: F) -> T {
    let waker = waker_ref(&AlwaysWake::INSTANCE);
    let mut cx = Context::from_waker(&waker);
    loop {
        if let Poll::Ready(t) = f(&mut cx) {
            return t;
        }
    }
}

pub fn poll_on<T, Fut: Future<Output = T>>(f: Fut) -> Poll<Fut::Output> {
    futures::pin_mut!(f);
    poll_fn(|cx| f.as_mut().poll(cx))
}


pub fn block_on<T, Fut: Future<Output = T>>(f: Fut) -> Fut::Output {
    futures::pin_mut!(f);
    block_fn(|cx| f.as_mut().poll(cx))
}