use std::{
    any::Any,
    cell::{Ref, RefCell, RefMut},
    fmt,
    future::Future,
    rc::{Rc, Weak},
};

use super::gen_vec::GenIdx;

pub type BoxedAnyFuture = Box<dyn Future<Output = Box<dyn Any + 'static>>>;
pub type TaskCompletionFn = Box<dyn FnOnce(Box<dyn Any>)>;
pub type TaskId = GenIdx;

/// A unit of work that can be scheduled and executed by the runtime.
///
/// In WASI-specific code, futures that require IO are scheduled by handing off
/// [WASI I/O](https://github.com/WebAssembly/wasi-io) pollables to the runtime. The runtime relies
/// on the WASI I/O poller (the `poll(pollables[])` function) to idle and wake up the program when
/// tasks are ready to continue execution. The mechanism to do this is [`super::Runtime::new_waker()`]
///
/// This structure is meant to be used internally to keep track of the task's state (`future`) and
/// what happens when it completes (`handler`).
pub struct Task {
    pub(super) future: BoxedAnyFuture,
    pub(super) handler: Option<TaskCompletionFn>,
}

/// An owning handle to a [`Task`].
#[derive(Debug, Clone)]
pub struct TaskHandle {
    inner: Rc<RefCell<Task>>,
}

/// A weak handle to a [`Task`]. This will not keep the task alive, and will return `None` if the
/// task has been dropped.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct WeakTaskHandle {
    inner: Weak<RefCell<Task>>,
}

impl fmt::Debug for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Task").finish_non_exhaustive()
    }
}

impl TaskHandle {
    pub fn new(task: Task) -> Self {
        let inner = Rc::new(RefCell::new(task));
        TaskHandle { inner }
    }

    #[allow(unused)]
    pub fn inner(&self) -> Ref<Task> {
        self.inner.borrow()
    }

    pub fn inner_mut(&self) -> RefMut<Task> {
        self.inner.borrow_mut()
    }

    #[allow(dead_code)]
    pub fn downgrade(&self) -> WeakTaskHandle {
        WeakTaskHandle {
            inner: Rc::downgrade(&self.inner),
        }
    }
}

impl WeakTaskHandle {
    #[allow(dead_code)]
    pub fn upgrade(&self) -> Option<TaskHandle> {
        self.inner.upgrade().map(|inner| TaskHandle { inner })
    }
}
