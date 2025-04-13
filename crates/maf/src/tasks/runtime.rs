use std::{
    any::Any,
    cell::{Ref, RefCell, RefMut},
    collections::VecDeque,
    future::{Future, IntoFuture},
    marker::PhantomData,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

use wasi::io::poll::Pollable;

use super::{
    gen_vec::GenVec,
    task::{Task, TaskHandle, TaskId},
    waker,
};

#[doc(hidden)]
pub static GLOBAL_RUNTIME: GlobalRuntime = GlobalRuntime::new();

#[repr(transparent)]
pub struct GlobalRuntime(RefCell<Option<Rc<Runtime>>>);

unsafe impl Sync for GlobalRuntime {}

impl GlobalRuntime {
    pub const fn new() -> Self {
        Self(RefCell::new(None))
    }

    pub fn set(&self, runtime: Rc<Runtime>) {
        self.0.replace(Some(runtime));
    }

    pub fn get(&self) -> Option<Rc<Runtime>> {
        self.0.borrow().clone()
    }
}

#[derive(Debug, Clone)]
pub struct Runtime {
    inner: Rc<RefCell<RuntimeInner>>,
}

#[derive(Debug)]
pub struct RuntimeInner {
    tasks: GenVec<TaskHandle>,
    new_tasks: VecDeque<TaskId>,
    pollables: Vec<(Pollable, Waker)>,
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(RuntimeInner {
                tasks: GenVec::new(),
                new_tasks: VecDeque::new(),
                pollables: Vec::new(),
            })),
        }
    }

    pub(crate) fn inner(&self) -> Ref<RuntimeInner> {
        self.inner.borrow()
    }

    pub(crate) fn inner_mut(&self) -> RefMut<RuntimeInner> {
        self.inner.borrow_mut()
    }

    fn task(&self, task_id: TaskId) -> TaskHandle {
        self.inner()
            .tasks
            .get(task_id)
            .expect("task not found")
            .clone()
    }

    // Used by external code to signal that a task is ready to be processed.
    pub fn resume_task(&self, task_id: TaskId) -> anyhow::Result<()> {
        let task = self
            .inner()
            .tasks
            .get(task_id)
            .ok_or_else(|| anyhow::anyhow!("task with id {:?} not found", task_id))?
            .clone();
        let mut task = task.inner_mut();

        let fut = unsafe { Pin::new_unchecked(task.future.as_mut()) };
        let waker = waker::create_waker(self.clone(), task_id);
        let mut ctx = std::task::Context::from_waker(&waker);

        match fut.poll(&mut ctx) {
            Poll::Ready(output) => {
                task.handler.take().map(|handler| handler(output));
                self.inner_mut().tasks.remove(task_id);
            }
            Poll::Pending => {}
        }

        Ok(())
    }

    pub fn blocking_poll(&self) {
        loop {
            loop {
                let new_tasks = self.inner_mut().new_tasks.drain(..).collect::<Vec<_>>();
                if new_tasks.is_empty() {
                    break;
                }
                for task_id in new_tasks {
                    self.resume_task(task_id).expect("failed to resume task");
                }
            }

            let inner = self.inner();
            let pollable_ref = inner
                .pollables
                .as_slice()
                .iter()
                .map(|(p, _)| &*p)
                .collect::<Vec<_>>();

            if pollable_ref.is_empty() {
                break;
            }

            let ready_poll_indices = wasi::io::poll::poll(&pollable_ref);
            drop(inner);

            for index in ready_poll_indices {
                let waker = {
                    let inner = self.inner();
                    let (_, waker_ref) = &inner.pollables[index as usize];
                    waker_ref.clone() // End the borrow of inner before calling wake_by_ref
                };

                waker.wake_by_ref();

                self.inner_mut().pollables.swap_remove(index as usize);
            }
        }
    }

    pub fn add_pollable(&self, pollable: Pollable, waker: Waker) {
        self.inner_mut().pollables.push((pollable, waker));
    }

    pub fn spawn<F: IntoFuture + 'static>(&self, fut: F) -> JoinHandle<F::Output>
    where
        F::Output: 'static,
    {
        let future = Box::new(async move {
            let result = fut.into_future().await;
            Box::new(result) as Box<dyn Any + 'static>
        });

        let id = self.inner_mut().tasks.push(TaskHandle::new(Task {
            future,
            handler: None,
        }));

        // println!("task {:?} spawned", id);

        // if self.get_task(id).is_none() {
        // println!("task {:?} finished immediately", id);
        // }

        self.inner_mut().new_tasks.push_back(id);

        JoinHandle {
            runtime: self.clone(),
            task_id: id,
            _phantom: PhantomData,
        }
    }

    pub fn current() -> Runtime {
        Runtime::clone(
            GLOBAL_RUNTIME
                .get()
                .expect("no global runtime set")
                .as_ref(),
        )
    }

    pub fn new_waker(cx: &std::task::Context, pollable: Pollable) {
        Self::current().add_pollable(pollable, cx.waker().clone());
    }

    pub fn global(self) {
        GLOBAL_RUNTIME.set(Rc::new(self));
    }
}

pub struct JoinHandle<T> {
    runtime: Runtime,
    task_id: TaskId,
    _phantom: PhantomData<T>,
}

impl<T: 'static> JoinHandle<T> {
    pub fn on_finish(self, f: impl FnOnce(T) + 'static) {
        let handler = Box::new(move |output: Box<dyn Any>| {
            let output = output.downcast::<T>().expect("output downcast failed");
            f(*output);
        });

        self.runtime.task(self.task_id).inner_mut().handler = Some(handler);
    }
}

impl<T: 'static> IntoFuture for JoinHandle<T> {
    type IntoFuture = JoinHandleFuture<T>;
    type Output = T;

    fn into_future(self) -> Self::IntoFuture {
        JoinHandleFuture {
            runtime: self.runtime,
            task_id: self.task_id,
            output: Rc::new(RefCell::new(None)),
            _phantom: PhantomData,
        }
    }
}

pub struct JoinHandleFuture<T> {
    runtime: Runtime,
    task_id: TaskId,
    output: Rc<RefCell<Option<T>>>,
    _phantom: PhantomData<T>,
}

impl<T: 'static> Future for JoinHandleFuture<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let task = self.runtime.task(self.task_id);

        match &mut task.inner_mut().handler {
            Some(handler) => {
                let waker = cx.waker().clone();
                let output_cell = self.output.clone();
                *handler = Box::new(move |output: Box<dyn Any>| {
                    let output = output.downcast::<T>().expect("output downcast failed");
                    output_cell.borrow_mut().replace(*output);
                    waker.wake_by_ref();
                });
            }
            None => {}
        }

        match self.output.borrow_mut().take() {
            Some(output) => Poll::Ready(output),
            None => Poll::Pending,
        }
    }
}
