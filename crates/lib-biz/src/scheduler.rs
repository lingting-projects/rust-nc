use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};

use lib_db::DbContext;
use tokio::runtime::Handle;

/// 并行任务数量上限。
const MAX_TASKS: usize = 5;

/// 调度器执行的同步任务。
///
/// 每个业务域实现一份: 只负责定义线程名称与单个 id 的同步流程,
/// 排队、并行上限与完成通知由 [`Scheduler`] 统一处理。
pub(crate) trait SyncTask: Send + Sync + 'static {
    /// 调度线程名称, 便于在日志与调试器里定位。
    const THREAD: &'static str;

    /// 同步单个 id, 失败原因由实现自行记录日志。
    fn run(context: Arc<DbContext>, id: String) -> impl Future<Output = ()> + Send;
}

/// 按并行上限执行同步任务的调度器。
///
/// 使用前必须通过 [`init`](Self::init) 注入数据库上下文与异步运行时。
/// 内部的主线程在推送与任务完成时被唤醒, 按并行上限的差额提交任务。
pub(crate) struct Scheduler<T> {
    state: Arc<State>,
    marker: std::marker::PhantomData<T>,
}

impl<T: SyncTask> Scheduler<T> {
    /// 创建调度器并启动主线程。
    pub(crate) fn new() -> Self {
        let state = Arc::new(State::default());
        let worker = state.clone();
        std::thread::Builder::new()
            .name(T::THREAD.to_string())
            .spawn(move || dispatch::<T>(worker))
            .expect("同步调度线程启动失败");

        Self {
            state,
            marker: std::marker::PhantomData,
        }
    }

    /// 注入任务所需的数据库上下文与异步运行时, 重复调用会覆盖旧值。
    pub(crate) fn init(&self, context: Arc<DbContext>, runtime: Handle) {
        {
            let mut inner = self.state.lock();
            inner.context = Some(context);
            inner.runtime = Some(runtime);
        }
        self.state.wake.notify_all();
    }

    /// 推送单个 id, 返回是否成功入队。
    ///
    /// 已在运行或已在队列中的 id 不会重复推入。
    pub(crate) fn push(&self, id: &str) -> bool {
        if id.is_empty() {
            return false;
        }

        let pushed = {
            let mut inner = self.state.lock();
            let queued = inner.queue.iter().any(|task| task.id == id);
            if inner.running.contains_key(id) || queued {
                false
            } else {
                inner.queue.push_back(Task::new(id));
                true
            }
        };

        if pushed {
            self.state.wake.notify_all();
        }
        pushed
    }

    /// 批量推送 id, 返回成功入队的数量。
    pub(crate) fn push_all(&self, ids: impl IntoIterator<Item = String>) -> usize {
        ids.into_iter().filter(|id| self.push(id)).count()
    }

    /// 正在运行或排队中的任务 id。
    ///
    /// 供界面判断「刷新中」: 只读共享状态, 不阻塞也不触发 IO。
    pub(crate) fn pending(&self) -> Vec<String> {
        let inner = self.state.lock();
        inner
            .running
            .keys()
            .chain(inner.queue.iter().map(|task| &task.id))
            .cloned()
            .collect()
    }

    /// 等待指定 id 的同步任务结束, 没有对应任务时直接完成。
    pub(crate) async fn wait(&self, id: &str) {
        let Some(slot) = self.slot(id) else {
            return;
        };

        // 任务运行在运行时线程上, 等待放到阻塞线程, 避免占住调用方的异步线程
        let _ = tokio::task::spawn_blocking(move || slot.wait()).await;
    }

    /// 指定 id 的等待句柄, 运行中或队列中的任务都有。
    fn slot(&self, id: &str) -> Option<Arc<WaitSlot>> {
        let inner = self.state.lock();
        inner.running.get(id).cloned().or_else(|| {
            inner
                .queue
                .iter()
                .find(|task| task.id == id)
                .map(Task::slot)
        })
    }
}

/// 主线程: 在推送与任务完成时被唤醒, 按并行上限的差额提交任务。
fn dispatch<T: SyncTask>(state: Arc<State>) {
    loop {
        // 取出任务与登记运行状态必须在同一把锁内完成, 否则重复推送可能挤进空档
        let (id, context, runtime) = {
            let mut inner = state.lock();
            loop {
                // 运行数量与上限的差额即本次可新增的任务数量, 因此每轮只取一个任务
                if inner.running.len() < MAX_TASKS
                    && let Some(task) = inner.queue.pop_front()
                {
                    match (inner.context.clone(), inner.runtime.clone()) {
                        (Some(context), Some(runtime)) => {
                            inner.running.insert(task.id.clone(), task.slot);
                            break (task.id, context, runtime);
                        }
                        // 未初始化时放回队列, 等待 init 唤醒
                        _ => inner.queue.push_front(task),
                    }
                }

                inner = state
                    .wake
                    .wait(inner)
                    .unwrap_or_else(|error| error.into_inner());
            }
        };

        let worker = state.clone();
        runtime.spawn(async move {
            // 运行状态由守卫清理：任务 panic 时后面的代码不会执行，
            // 若不用守卫，该 id 会永久留在运行集合里，后续推送全部被拒绝。
            let task = RunningGuard { state: worker, id };
            T::run(context, task.id.clone()).await;
        });
    }
}

/// 运行状态守卫：任务结束或 panic 时都会把 id 从运行集合中移除并发出完成信号。
struct RunningGuard {
    state: Arc<State>,
    id: String,
}

impl Drop for RunningGuard {
    fn drop(&mut self) {
        let slot = self.state.lock().running.remove(&self.id);
        if let Some(slot) = slot {
            slot.finish();
        }
        // 运行数量减少, 唤醒主线程补齐任务
        self.state.wake.notify_all();
    }
}

/// 同步任务。
struct Task {
    id: String,
    /// 任务完成信号
    slot: Arc<WaitSlot>,
}

impl Task {
    fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            slot: Arc::new(WaitSlot::default()),
        }
    }

    fn slot(&self) -> Arc<WaitSlot> {
        self.slot.clone()
    }
}

/// 任务完成信号, 供 [`Scheduler::wait`] 等待。
#[derive(Default)]
struct WaitSlot {
    done: Mutex<bool>,
    signal: Condvar,
}

impl WaitSlot {
    /// 标记任务结束并唤醒等待方。
    fn finish(&self) {
        *self.done.lock().unwrap_or_else(|error| error.into_inner()) = true;
        self.signal.notify_all();
    }

    /// 阻塞等待任务结束。
    fn wait(&self) {
        let mut done = self.done.lock().unwrap_or_else(|error| error.into_inner());
        while !*done {
            done = self
                .signal
                .wait(done)
                .unwrap_or_else(|error| error.into_inner());
        }
    }
}

/// 调度器共享状态。
#[derive(Default)]
struct State {
    inner: Mutex<Inner>,
    /// 推送与任务完成时的唤醒信号
    wake: Condvar,
}

impl State {
    /// 取共享状态, 锁被污染时继续使用其内容。
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|error| error.into_inner())
    }
}

/// 调度器内部数据。
#[derive(Default)]
struct Inner {
    /// 任务所需的数据库上下文, 未初始化时为 `None`
    context: Option<Arc<DbContext>>,
    /// 任务所需的异步运行时, 未初始化时为 `None`
    runtime: Option<Handle>,
    /// 待运行的任务
    queue: VecDeque<Task>,
    /// 运行中的任务
    running: HashMap<String, Arc<WaitSlot>>,
}
