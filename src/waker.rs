use core::mem::ManuallyDrop;
use core::sync::atomic::{AtomicBool, Ordering};
use futures::task::WakerRef;
use core::task::{Waker, RawWaker, RawWakerVTable};

#[allow(dead_code)]
#[derive(Debug)]
pub struct SingleWake {
    woken: AtomicBool,
}

#[allow(dead_code)]
impl SingleWake {
    pub fn new() -> Self {
        Self {
            woken: Default::default()
        }
    }
    pub fn read_reset(&self) -> bool {
        self.woken.fetch_and(false, Ordering::Relaxed)
    }
}

impl SimpleWaker for SingleWake {
    fn wake(&self) {
        self.woken.store(true, Ordering::Relaxed)
    }
}

pub struct AlwaysWake {}

impl AlwaysWake {
    pub const INSTANCE: AlwaysWake = Self::new();
    pub const fn new() -> Self {
        Self {}
    }
}

impl SimpleWaker for AlwaysWake {
    fn wake(&self) {}
}

pub(super) trait SimpleWaker {
    fn wake(&self);
}

/// Creates a reference to a [`Waker`] from a reference to `Arc<impl ArcWake>`.
///
/// The resulting [`Waker`] will call
/// [`ArcWake.wake()`](ArcWake::wake) if awoken.
#[inline]
pub(crate) fn waker_ref<W>(wake: &W) -> WakerRef<'_>
    where
        W: SimpleWaker,
{
    let ptr = (wake as *const W) as *const ();

    let waker =
        ManuallyDrop::new(unsafe { Waker::from_raw(RawWaker::new(ptr, waker_vtable::<W>())) });
    WakerRef::new_unowned(waker)
}

pub(super) fn waker_vtable<W: SimpleWaker>() -> &'static RawWakerVTable {
    &RawWakerVTable::new(
        copy_ref_raw::<W>,
        wake_raw::<W>,
        wake_by_ref_raw::<W>,
        drop_raw::<W>,
    )
}


unsafe fn copy_ref_raw<T: SimpleWaker>(data: *const ()) -> RawWaker {
    RawWaker::new(data, waker_vtable::<T>())
}

unsafe fn wake_raw<T: SimpleWaker>(data: *const ()) {
    let data = core::ptr::read(data as *const T);
    SimpleWaker::wake(&data);
    drop(data)
}

unsafe fn wake_by_ref_raw<T: SimpleWaker>(data: *const ()) {
    let data = &*(data as *const T);
    SimpleWaker::wake(data);
}

unsafe fn drop_raw<T: SimpleWaker>(data: *const ()) {
    let data = core::ptr::read(data as *const T);
    drop(data)
}
