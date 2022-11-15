use alloc::sync::{Arc, Weak};
use futures::future::LocalFutureObj;
use futures::{FutureExt};
use core::task::{Poll};
use crossbeam::queue::ArrayQueue;
use futures::task::UnsafeFutureObj;
use crate::poll_fn;
use futures::future::FutureObj;
use futures::task::Spawn;
use futures::task::SpawnError;

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
pub struct LocalPool<'a, Ret = ()> {
    pool: Arc<ArrayQueue<LocalFutureObj<'a, Ret>>>,
}


#[derive(Clone)]
pub struct Spawner<'a, Ret> {
    tx: Weak<ArrayQueue<LocalFutureObj<'a, Ret>>>,
}


impl<'a> Spawner<'a, ()> {
    pub fn spawn<F>(&self, f: F) -> Result<(), SpawnError>
        where F: UnsafeFutureObj<'a, ()> + Send {
        let tx = self.tx.upgrade().ok_or(SpawnError::shutdown())?;
        tx.push(LocalFutureObj::new(f)).expect("Queue full");
        Ok(())
    }
}


impl Spawn for Spawner<'static, ()> {
    fn spawn_obj(&self, future: FutureObj<'static, ()>) -> Result<(), SpawnError> {
        let tx = self.tx.upgrade().ok_or(SpawnError::shutdown())?;
        tx.push(future.into()).expect("Queue full");
        Ok(())
    }
}


impl<'a, Ret> LocalPool<'a, Ret> {
    /// Create a new, empty pool of tasks.
    pub fn new(cap: usize) -> Self {
        Self {
            pool: Arc::new(ArrayQueue::new(cap)),
        }
    }

    pub fn spawner(&self) -> Spawner<'a, Ret> {
        Spawner {
            tx: Arc::downgrade(&self.pool),
        }
    }
    pub fn spawn<F>(&mut self, f: F)
        where F: UnsafeFutureObj<'a, Ret> {
        self.pool.push(LocalFutureObj::new(f)).expect("Queue full");
    }
    /// Run all tasks in the pool to completion.
    ///
    /// ```rust
    ///
    /// use minimal_executor::LocalPool;
    ///
    /// let mut pool: LocalPool<'_, ()> = LocalPool::new();
    ///
    /// // ... spawn some initial tasks using `spawn.spawn()` or `spawn.spawn_local()`
    ///
    /// // run *all* tasks in the pool to completion, including any newly-spawned ones.
    /// pool.run();
    /// ```
    ///
    /// The function will block the calling thread until *all* tasks in the pool
    /// are complete, including any spawned while running existing tasks.
    pub fn run(&mut self) -> alloc::vec::Vec<Ret> {
        let mut results = alloc::vec::Vec::new();
        loop {
            let ret = self.poll_once();

            // no queued tasks; we may be done
            match ret {
                Poll::Pending => {}
                Poll::Ready(None) => break,
                Poll::Ready(Some(r)) => { results.push(r); }
            }
        }
        results
    }

    /// Runs all tasks and returns after completing one future or until no more progress
    /// can be made. Returns `true` if one future was completed, `false` otherwise.
    ///
    /// ```rust
    ///
    /// use futures::task::LocalSpawnExt;
    /// use futures::future::{ready, pending};
    /// use minimal_executor::LocalPool;
    ///
    /// let mut pool: LocalPool<'_, ()> = LocalPool::new();
    /// pool.spawn(Box::pin(ready(())));
    /// pool.spawn(Box::pin(ready(())));
    /// pool.spawn(Box::pin(pending()));
    ///
    /// // Run the two ready tasks and return true for them.
    /// pool.try_run_one(); // returns true after completing one of the ready futures
    /// pool.try_run_one(); // returns true after completing the other ready future
    ///
    /// // the remaining task can not be completed
    /// assert!(pool.try_run_one().is_pending()); // returns false
    /// ```
    ///
    /// This function will not block the calling thread and will return the moment
    /// that there are no tasks left for which progress can be made or after exactly one
    /// task was completed; Remaining incomplete tasks in the pool can continue with
    /// further use of one of the pool's run or poll methods.
    /// Though only one task will be completed, progress may be made on multiple tasks.
    pub fn try_run_one(&mut self) -> Poll<Ret> {
        let ret = self.poll_though();
        match ret {
            Poll::Ready(Some(ret)) => {
                Poll::Ready(ret)
            }
            Poll::Ready(None) => {
                Poll::Pending
            }
            Poll::Pending => {
                Poll::Pending
            }
        }
    }

    pub fn poll_though(&mut self) -> Poll<Option<Ret>> {
        let len = self.pool.len();
        if len == 0 {
            return Poll::Ready(None);
        }
        poll_fn(|cx| {
            for _ in 0..len {
                if let Some(mut future) = self.pool.pop() {
                    match future.poll_unpin(cx) {
                        Poll::Pending => {
                            self.pool.push(future).expect("Queue full");
                        }
                        Poll::Ready(ret) => {
                            return Poll::Ready(Some(ret));
                        }
                    }
                }
            }
            Poll::Pending
        })
    }
    pub fn poll_once(&mut self) -> Poll<Option<Ret>> {
        if let Some(mut future) = self.pool.pop() {
            match poll_fn(|cx| future.poll_unpin(cx)) {
                Poll::Pending => {
                    self.pool.push(future).expect("Queue full");
                    Poll::Pending
                }
                Poll::Ready(ret) => {
                    Poll::Ready(Some(ret))
                }
            }
        } else {
            Poll::Ready(None)
        }
    }
}

impl<'a, Ret> Default for LocalPool<'a, Ret> {
    fn default() -> Self {
        Self::new()
    }
}

