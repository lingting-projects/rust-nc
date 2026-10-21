use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};

use anyhow::{Context as _, Result};

use crate::SingBoxGrpc;

/// 测速任务队列。
///
/// 队列只有一个消费者：一个后台线程按顺序取出节点并触发一次内核测速，
/// 内核再按自己的节奏把新的延迟推给订阅方。因此这里不并发，也不会因为
/// 同时点多次测速而把内核压满。
///
/// 推送方通过 [`push`](Self::push) 提交节点，同一个节点在队列中或正在
/// 测速时不会被重复推入，返回 `false` 即表示「已经在排队了」。
pub struct UrlTestQueue {
    state: Arc<State>,
}

impl UrlTestQueue {
    /// 创建队列并启动消费线程。
    pub fn new() -> Self {
        let state = Arc::new(State::default());
        let worker = state.clone();
        std::thread::Builder::new()
            .name("singbox-url-test".to_string())
            .spawn(move || dispatch(worker))
            .expect("测速线程启动失败");

        Self { state }
    }

    /// 提交一个待测速的节点，返回是否成功入队。
    ///
    /// 队列中已有的节点、正在测速的节点都会被跳过，避免重复触发。
    pub fn push(&self, tag: &str) -> bool {
        if tag.is_empty() {
            return false;
        }

        let pushed = {
            let mut inner = self.state.lock();
            if inner.running.as_deref() == Some(tag) || inner.queue.iter().any(|item| item == tag) {
                false
            } else {
                inner.queue.push_back(tag.to_string());
                true
            }
        };

        if pushed {
            self.state.wake.notify_all();
        }
        pushed
    }

    /// 队列中等待测速的节点数量，供界面展示。
    pub fn pending(&self) -> usize {
        let inner = self.state.lock();
        inner.queue.len() + usize::from(inner.running.is_some())
    }

    /// 注入内核客户端，传入 `None` 表示内核已不可用。
    ///
    /// 内核启停时由 [`SingBox`](crate::SingBox) 调用：客户端为空时消费线程只等待，
    /// 队列内容在下次内核运行后继续执行。
    pub fn set_grpc(&self, grpc: Option<Arc<SingBoxGrpc>>) {
        self.state.lock().grpc = grpc;
        self.state.wake.notify_all();
    }

    /// 丢弃全部待测速节点。
    ///
    /// 内核停止后旧队列已经没有意义，继续测速只会得到连接错误。
    pub fn clear(&self) {
        self.state.lock().queue.clear();
    }
}

impl Default for UrlTestQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// 消费线程：有任务时取出一个同步测速，没有任务时等待唤醒。
fn dispatch(state: Arc<State>) {
    let runtime = match runtime() {
        Ok(runtime) => runtime,
        Err(error) => {
            log::error!("[内核] 测速线程启动失败: {error:#}");
            return;
        }
    };

    loop {
        let (tag, grpc) = {
            let mut inner = state.lock();
            loop {
                if let (Some(tag), Some(grpc)) = (inner.queue.pop_front(), inner.grpc.clone()) {
                    inner.running = Some(tag.clone());
                    break (tag, grpc);
                }

                // 没有任务或内核客户端尚未注入时等待，注入方会唤醒
                inner = state
                    .wake
                    .wait(inner)
                    .unwrap_or_else(|error| error.into_inner());
            }
        };

        if let Err(error) = runtime.block_on(async { grpc.url_test(&tag).await }) {
            log::warn!("[内核] 节点测速失败: {tag}: {error:#}");
        }

        // 运行状态必须在测速结束后清掉，否则该节点后续推送会一直被拒绝
        state.lock().running = None;
        state.wake.notify_all();
    }
}

/// 测速专用的异步运行时。
///
/// 消费线程是普通线程，请求需要运行时才能驱动；这里自建一个单线程运行时，
/// 不必让调用方把运行时句柄传进来。
fn runtime() -> Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("创建测速运行时失败")
}

/// 队列共享状态。
#[derive(Default)]
struct State {
    inner: Mutex<Inner>,
    /// 推送与消费结束时的唤醒信号
    wake: Condvar,
}

impl State {
    /// 取共享状态，锁被污染时继续使用其内容。
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|error| error.into_inner())
    }
}

/// 队列内部数据。
#[derive(Default)]
struct Inner {
    /// 内核客户端，内核未运行时为空
    grpc: Option<Arc<SingBoxGrpc>>,
    /// 待测速的节点
    queue: VecDeque<String>,
    /// 正在测速的节点
    running: Option<String>,
}
