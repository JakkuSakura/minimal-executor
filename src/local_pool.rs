use futures::stream::FuturesUnordered;
use futures::future::LocalFutureObj;
use futures::StreamExt;
use std::future::Future;
use std::task::{Poll, Context};
use crate::waker::{AlwaysWake, waker_ref};

pub fn poll_executor<T, F: FnMut(&mut Context<'_>) -> T>(mut f: F) -> T {
    let waker = waker_ref(&AlwaysWake::INSTANCE);
    let mut cx = Context::from_waker(&waker);
    f(&mut cx)
}

// Set up and run a basic single-threaded spawner loop, invoking `f` on each
// turn.
fn run_executor<T, F: FnMut(&mut Context<'_>) -> Poll<T>>(mut f: F) -> T {
    let waker = waker_ref(&AlwaysWake::INSTANCE);
    let mut cx = Context::from_waker(&waker);
    loop {
        if let Poll::Ready(t) = f(&mut cx) {
            return t;
        }
    }
}

pub fn poll_on<F: Future>(f: F) -> Poll<F::Output> {
    futures::pin_mut!(f);
    poll_executor(|cx| f.as_mut().poll(cx))
}

/// A single-threaded task pool for polling futures to completion.
///
/// This executor allows you to multiplex any number of tasks onto a single
/// thread. It's appropriate to poll strictly I/O-bound futures that do very
/// little work in between I/O actions.
///
/// To get a handle to the pool that implements
/// [`Spawn`](futures_task::Spawn), use the
/// [`spawner()`](LocalPool::spawner) method. Because the executor is
/// single-threaded, it supports a special form of task spawning for non-`Send`
/// futures, via [`spawn_local_obj`](futures_task::LocalSpawn::spawn_local_obj).
#[derive(Debug)]
pub struct LocalPool<'a> {
    pool: FuturesUnordered<LocalFutureObj<'a, ()>>,
}

impl<'a> LocalPool<'a> {
    /// Create a new, empty pool of tasks.
    pub fn new() -> Self {
        Self { pool: FuturesUnordered::new() }
    }

    pub fn spawn<F>(&mut self, f: F)
        where LocalFutureObj<'a, ()>: From<F> {
        self.pool.push(f.into())
    }
    /// Run all tasks in the pool to completion.
    ///
    /// ```
    /// use futures::executor::LocalPool;
    ///
    /// let mut pool = LocalPool::new();
    ///
    /// // ... spawn some initial tasks using `spawn.spawn()` or `spawn.spawn_local()`
    ///
    /// // run *all* tasks in the pool to completion, including any newly-spawned ones.
    /// pool.run();
    /// ```
    ///
    /// The function will block the calling thread until *all* tasks in the pool
    /// are complete, including any spawned while running existing tasks.
    pub fn run(&mut self) {
        run_executor(|cx| self.poll_pool(cx))
    }

    /// Runs all tasks and returns after completing one future or until no more progress
    /// can be made. Returns `true` if one future was completed, `false` otherwise.
    ///
    /// ```
    /// use futures::executor::LocalPool;
    /// use futures::task::LocalSpawnExt;
    /// use futures::future::{ready, pending};
    ///
    /// let mut pool = LocalPool::new();
    /// let spawner = pool.spawner();
    ///
    /// spawner.spawn_local(ready(())).unwrap();
    /// spawner.spawn_local(ready(())).unwrap();
    /// spawner.spawn_local(pending()).unwrap();
    ///
    /// // Run the two ready tasks and return true for them.
    /// pool.try_run_one(); // returns true after completing one of the ready futures
    /// pool.try_run_one(); // returns true after completing the other ready future
    ///
    /// // the remaining task can not be completed
    /// assert!(!pool.try_run_one()); // returns false
    /// ```
    ///
    /// This function will not block the calling thread and will return the moment
    /// that there are no tasks left for which progress can be made or after exactly one
    /// task was completed; Remaining incomplete tasks in the pool can continue with
    /// further use of one of the pool's run or poll methods.
    /// Though only one task will be completed, progress may be made on multiple tasks.
    pub fn try_run_one(&mut self) -> bool {
        poll_executor(|ctx| {
            let ret = self.pool.poll_next_unpin(ctx);
            match ret {
                Poll::Ready(Some(())) => {
                    true
                }
                Poll::Ready(None) => {
                    false
                }
                Poll::Pending => {
                    false
                }
            }
        })
    }

    /// Runs all tasks in the pool and returns if no more progress can be made
    /// on any task.
    ///
    /// ```
    /// use futures::executor::LocalPool;
    /// use futures::task::LocalSpawnExt;
    /// use futures::future::{ready, pending};
    ///
    /// let mut pool = LocalPool::new();
    /// let spawner = pool.spawner();
    ///
    /// spawner.spawn_local(ready(())).unwrap();
    /// spawner.spawn_local(ready(())).unwrap();
    /// spawner.spawn_local(pending()).unwrap();
    ///
    /// // Runs the two ready task and returns.
    /// // The empty task remains in the pool.
    /// pool.run_until_stalled();
    /// ```
    ///
    /// This function will not block the calling thread and will return the moment
    /// that there are no tasks left for which progress can be made;
    /// remaining incomplete tasks in the pool can continue with further use of one
    /// of the pool's run or poll methods. While the function is running, all tasks
    /// in the pool will try to make progress.
    pub fn run_until_stalled(&mut self) {
        poll_executor(|ctx| {
            let _ = self.poll_pool(ctx);
        });
    }

    // Make maximal progress on the entire pool of spawned task, returning `Ready`
    // if the pool is empty and `Pending` if no further progress can be made.
    fn poll_pool(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        // state for the FuturesUnordered, which will never be used
        loop {
            let ret = self.pool.poll_next_unpin(cx);

            // no queued tasks; we may be done
            match ret {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(None) => return Poll::Ready(()),
                _ => {}
            }
        }
    }

    pub fn poll_once(&mut self) {
        let _ = poll_executor(|cx| {
            self.pool.poll_next_unpin(cx)
        });
    }
}

impl<'a> Default for LocalPool<'a> {
    fn default() -> Self {
        Self::new()
    }
}

