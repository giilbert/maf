use std::{
    rc::Rc,
    task::{RawWaker, RawWakerVTable, Waker},
};

struct WakerData {}

impl WakerData {
    pub unsafe fn from_raw(raw: *const ()) -> Rc<Self> {
        Rc::from_raw(raw as *const Self)
    }
}

fn create_raw_waker(data: Rc<WakerData>) -> RawWaker {
    RawWaker::new(
        Rc::into_raw(data) as *const (),
        &RawWakerVTable::new(
            clone_callback,
            wake_callback,
            wake_by_ref_callback,
            drop_callback,
        ),
    )
}

pub(super) fn create_waker() -> Waker {
    let data = Rc::new(WakerData {});
    let waker = unsafe { Waker::from_raw(create_raw_waker(data)) };
    waker
}

unsafe fn clone_callback(ptr: *const ()) -> RawWaker {
    let rc = WakerData::from_raw(ptr);
    let clone = Rc::clone(&rc);

    std::mem::forget(rc);
    RawWaker::new(
        Rc::into_raw(clone) as *const (),
        &RawWakerVTable::new(
            clone_callback,
            wake_callback,
            wake_by_ref_callback,
            drop_callback,
        ),
    )
}

unsafe fn wake_callback(ptr: *const ()) {
    let rc = WakerData::from_raw(ptr);

    todo!();

    std::mem::forget(rc);
}

unsafe fn wake_by_ref_callback(ptr: *const ()) {
    let rc = WakerData::from_raw(ptr);

    todo!();

    std::mem::forget(rc);
}

unsafe fn drop_callback(ptr: *const ()) {
    drop(WakerData::from_raw(ptr));
}
