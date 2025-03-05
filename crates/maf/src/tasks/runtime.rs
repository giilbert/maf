use std::{
    any::Any,
    cell::{Ref, RefCell, RefMut},
    future::{Future, IntoFuture},
    marker::PhantomData,
    pin::{pin, Pin},
    rc::Rc,
};

#[derive(Clone)]
pub struct WasmAsyncRuntime {
    inner: Rc<RefCell<WasmAsyncRuntimeInner>>,
}

#[derive(Debug)]
pub struct WasmAsyncRuntimeInner {
    tasks: Vec<(TaskId, Option<Rc<RefCell<Task>>>)>,
    free_task_ids: Vec<u32>,
}

pub struct Task {
    future: Box<dyn Future<Output = Box<dyn Any + 'static>>>,
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

impl WasmAsyncRuntime {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(WasmAsyncRuntimeInner {
                tasks: Vec::new(),
                free_task_ids: Vec::new(),
            })),
        }
    }

    pub(crate) fn inner(&self) -> Ref<WasmAsyncRuntimeInner> {
        self.inner.borrow()
    }

    pub(crate) fn inner_mut(&self) -> RefMut<WasmAsyncRuntimeInner> {
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

    // Used by external code to signal that a task is ready to be processed.
    pub fn ready_task(&self, task_id: TaskId) -> anyhow::Result<()> {
        let task = self
            .get_task(task_id)
            .ok_or_else(|| anyhow::anyhow!("task with id {:?} not found", task_id))?;

        let fut = unsafe { Pin::new_unchecked(&mut task.borrow_mut().future.as_mut()) };

        Ok(())
    }

    pub fn spawn<F: IntoFuture + 'static>(&self, fut: F) -> JoinHandle<F::Output>
    where
        F::Output: 'static,
    {
        let future = Box::new(async move {
            let result = fut.into_future().await;
            Box::new(result) as Box<dyn Any + 'static>
        });

        let id = self.push_task(Task { future });
        self.ready_task(id);

        JoinHandle {
            _phantom: PhantomData,
        }
    }
}

#[derive(Debug)]
pub struct JoinHandle<T> {
    _phantom: PhantomData<T>,
}
