use minimal_executor::LocalPool;
use futures::future::{lazy, Future};
use futures::task::{Context, Poll};
use std::cell::{Cell};
use std::pin::Pin;
use std::rc::Rc;
use futures::FutureExt;

struct Pending(Rc<()>);

impl Future for Pending {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        Poll::Pending
    }
}

fn pending() -> Pending {
    Pending(Rc::new(()))
}

#[test]
fn run_until_single_future() {
    let mut cnt = 0;

    {
        let mut pool = LocalPool::new();
        let fut = lazy(|_| {
            cnt += 1;
        });
        pool.spawn(fut.boxed_local());
        let _ = pool.poll_once();
    }

    assert_eq!(cnt, 1);
}

#[test]
fn run_returns_if_empty() {
    let mut pool: LocalPool<()> = LocalPool::new();
    pool.run();
    pool.run();
}

#[test]
fn run_spawn_many() {
    const ITER: usize = 200;

    let cnt = Rc::new(Cell::new(0));

    let mut pool = LocalPool::new();

    for _ in 0..ITER {
        let cnt = cnt.clone();
        pool.spawn(Box::pin(lazy(move |_| {
            cnt.set(cnt.get() + 1);
        })));
    }

    pool.run();

    assert_eq!(cnt.get(), ITER);
}

#[test]
fn try_run_one_returns_if_empty() {
    let mut pool: LocalPool<()> = LocalPool::new();
    assert!(pool.try_run_one().is_pending());
}

#[test]
fn try_run_one_executes_one_ready() {
    const ITER: usize = 200;

    let cnt = Rc::new(Cell::new(0));

    let mut pool = LocalPool::new();

    for _ in 0..ITER {
        pool.spawn(Box::pin(pending()));

        let cnt = cnt.clone();
        pool
            .spawn(
                Box::pin(lazy(move |_| {
                    cnt.set(cnt.get() + 1);
                })),
            );

        pool.spawn(Box::pin(pending()));
    }

    for i in 0..ITER {
        assert_eq!(cnt.get(), i);
        assert!(pool.try_run_one().is_ready());
        assert_eq!(cnt.get(), i + 1);
    }
    assert!(pool.try_run_one().is_pending());
}
