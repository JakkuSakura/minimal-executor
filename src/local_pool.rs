use futures::stream::FuturesUnordered;
use futures::future::LocalFutureObj;
use futures::StreamExt;
use core::task::{Poll};
use futures::task::UnsafeFutureObj;
use crate::poll_fn;
#[cfg(feature = "std")]
use futures::future::FutureObj;
#[cfg(feature = "std")]
use futures::task::Spawn;
#[cfg(feature = "std")]
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
    pool: FuturesUnordered<LocalFutureObj<'a, Ret>>,
    #[cfg(feature = "std")]
    rx: crossbeam::channel::Receiver<FutureObj<'static, Ret>>,
    #[cfg(feature = "std")]
    tx: crossbeam::channel::Sender<FutureObj<'static, Ret>>,
}

#[cfg(feature = "std")]
#[derive(Clone)]
pub struct Spawner<Ret> {
    tx: crossbeam::channel::Sender<FutureObj<'static, Ret>>,
}

#[cfg(feature = "std")]
impl<Ret> Spawner<Ret> {
    pub fn spawn<F>(&self, f: F) -> Result<(), SpawnError>
        where F: UnsafeFutureObj<'static, Ret> + Send {
        self.tx.send(FutureObj::new(f)).map_err(|_| SpawnError::shutdown())
    }
}

#[cfg(feature = "std")]
impl Spawn for Spawner<()> {
    fn spawn_obj(&self, future: FutureObj<'static, ()>) -> Result<(), SpawnError> {
        self.tx.send(future).map_err(|_| SpawnError::shutdown())
    }
}


impl<'a, Ret> LocalPool<'a, Ret> {
    /// Create a new, empty pool of tasks.
    pub fn new() -> Self {
        #[cfg(feature = "std")] {
            let (tx, rx) = crossbeam::channel::unbounded();
            Self { pool: FuturesUnordered::new(), rx, tx }
        }
        #[cfg(not(feature = "std"))] {
            Self { pool: FuturesUnordered::new() }
        }
    }
    #[cfg(feature = "std")]
    pub fn spawner(&self) -> Spawner<Ret> {
        Spawner {
            tx: self.tx.clone()
        }
    }
    pub fn spawn<F>(&mut self, f: F)
        where F: UnsafeFutureObj<'a, Ret> {
        self.pool.push(LocalFutureObj::new(f))
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
                Poll::Pending => {},
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
        let ret = self.poll_once();
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


    pub fn poll_once(&mut self) -> Poll<Option<Ret>> {
        poll_fn(|cx| {
            #[cfg(feature = "std")]
            while let Ok(fut) = self.rx.try_recv() {
                self.pool.push(LocalFutureObj::from(fut))
            }
            self.pool.poll_next_unpin(cx)
        })
    }
}

impl<'a, Ret> Default for LocalPool<'a, Ret> {
    fn default() -> Self {
        Self::new()
    }
}

