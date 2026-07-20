use std::rc::Rc;
use std::task::{RawWaker, RawWakerVTable, Waker};

use super::Runtime;
use super::task::TaskId;

struct WakerData {
    runtime: Runtime,
    task_id: TaskId,
}

impl WakerData {
    pub unsafe fn from_raw(raw: *const ()) -> Rc<Self> {
        unsafe { Rc::from_raw(raw as *const Self) }
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

pub(super) fn create_waker(runtime: Runtime, task_id: TaskId) -> Waker {
    let data = Rc::new(WakerData { runtime, task_id });
    let waker = unsafe { Waker::from_raw(create_raw_waker(data)) };
    waker
}

unsafe fn clone_callback(ptr: *const ()) -> RawWaker {
    let rc = unsafe { WakerData::from_raw(ptr) };
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
    let rc = unsafe { WakerData::from_raw(ptr) };

    rc.runtime
        .resume_task(rc.task_id)
        .expect("error polling task");

    std::mem::forget(rc);
}

unsafe fn wake_by_ref_callback(ptr: *const ()) {
    let rc = unsafe { WakerData::from_raw(ptr) };

    rc.runtime
        .resume_task(rc.task_id)
        .expect("error polling task");

    std::mem::forget(rc);
}

unsafe fn drop_callback(ptr: *const ()) {
    drop(unsafe { WakerData::from_raw(ptr) });
}
