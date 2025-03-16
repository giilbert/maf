use std::{
    any::Any,
    cell::{Ref, RefCell, RefMut},
    future::{Future, IntoFuture},
    marker::PhantomData,
    pin::Pin,
    rc::Rc,
    task::{Poll, Waker},
};

use wasi::io::poll::Pollable;

use super::waker;

#[derive(Debug, Clone)]
pub struct Runtime {
    inner: Rc<RefCell<RuntimeInner>>,
}

#[derive(Debug)]
pub struct RuntimeInner {
    tasks: Vec<(TaskId, Option<Rc<RefCell<Task>>>)>,
    free_task_ids: Vec<u32>,

    pollables: Vec<(Pollable, Waker)>,
}

pub struct Task {
    future: Box<dyn Future<Output = Box<dyn Any + 'static>>>,
    handler: Option<Box<dyn FnOnce(Box<dyn Any>)>>,
}

impl std::fmt::Debug for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Task").finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskId {
    generation: u32,
    index: u32,
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(RuntimeInner {
                tasks: Vec::new(),
                free_task_ids: Vec::new(),

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

    fn push_task(&self, task: Task) -> TaskId {
        let mut inner = self.inner_mut();

        if let Some(id) = inner.free_task_ids.pop() {
            let old_task_id = inner.tasks[id as usize].0;
            let new_task_id = TaskId {
                generation: old_task_id.generation + 1,
                index: id,
            };
            inner.tasks[id as usize] = (new_task_id, Some(Rc::new(RefCell::new(task))));

            new_task_id
        } else {
            let new_task_id = TaskId {
                generation: 0,
                index: inner.tasks.len() as u32,
            };
            inner
                .tasks
                .push((new_task_id, Some(Rc::new(RefCell::new(task)))));

            new_task_id
        }
    }

    fn remove_task(&self, task_id: TaskId) -> Option<Rc<RefCell<Task>>> {
        let mut inner = self.inner_mut();

        if task_id.index as usize >= inner.tasks.len() {
            return None;
        }

        let (old_task_id, task) = &mut inner.tasks[task_id.index as usize];
        if old_task_id.generation == task_id.generation {
            let task = task.take()?;
            inner.free_task_ids.push(task_id.index);
            Some(task)
        } else {
            None
        }
    }

    fn get_task(&self, task_id: TaskId) -> Option<Rc<RefCell<Task>>> {
        let inner = self.inner();
        if task_id.index as usize >= inner.tasks.len() {
            return None;
        }

        let (id, task) = &inner.tasks[task_id.index as usize];
        if id.generation == task_id.generation {
            task.clone()
        } else {
            None
        }
    }

    // pub(crate) fn waker_called(&self, task_id: TaskId) {
    //     let mut inner = self.inner_mut();
    //     inner.resumed_tasks.push_back(task_id);
    // }

    // Used by external code to signal that a task is ready to be processed.
    pub fn resume_task(&self, task_id: TaskId) -> anyhow::Result<()> {
        let task = self
            .get_task(task_id)
            .ok_or_else(|| anyhow::anyhow!("task with id {:?} not found", task_id))?;
        let mut task = task.borrow_mut();

        let fut = unsafe { Pin::new_unchecked(task.future.as_mut()) };
        let waker = waker::create_waker(self.clone(), task_id);
        let mut ctx = std::task::Context::from_waker(&waker);

        match fut.poll(&mut ctx) {
            Poll::Ready(output) => {
                task.handler.take().map(|handler| handler(output));
                self.remove_task(task_id);
            }
            Poll::Pending => {
                println!("task {:?} is pending. putting to sleep..", task_id);
            }
        }

        Ok(())
    }

    pub fn blocking_poll(&self) {
        loop {
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
                println!("pollable {:?} is ready", index);

                let waker = {
                    let inner = self.inner();
                    let (_, waker_ref) = &inner.pollables[index as usize];
                    waker_ref.clone() // End the borrow of inner before calling wake_by_ref
                };

                waker.wake_by_ref();

                let mut inner_mut = self.inner_mut();
                inner_mut.pollables.remove(index as usize);
            }
        }

        println!("blocking_poll finished");
    }

    pub fn add_pollable(&self, pollable: Pollable, waker: Waker) {
        println!("adding pollable");
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

        let id = self.push_task(Task {
            future,
            handler: None,
        });

        if self.get_task(id).is_none() {
            println!("task {:?} finished immediately", id);
        }

        JoinHandle {
            runtime: self.clone(),
            task_id: id,
            _phantom: PhantomData,
        }
    }
}

pub struct JoinHandle<T> {
    runtime: Runtime,
    task_id: TaskId,
    _phantom: PhantomData<T>,
}

impl<T: 'static> JoinHandle<T> {
    pub fn execute(self) {
        self.runtime
            .resume_task(self.task_id)
            .expect("error polling task");
    }

    pub fn on_finish(self, f: impl FnOnce(T) + 'static) {
        let handler = Box::new(move |output: Box<dyn Any>| {
            let output = output.downcast::<T>().expect("output downcast failed");
            f(*output);
        });

        self.runtime
            .get_task(self.task_id)
            .map(|task| task.borrow_mut().handler = Some(handler));

        self.execute();
    }
}
