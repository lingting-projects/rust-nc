//! 与监控线程共享的运行状态。

use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::Duration;

use arc_swap::{ArcSwap, ArcSwapOption};
use lib_system::process::Process;

use super::Phase;
use crate::connection::ConnectionStore;
use crate::grpc::SingBoxGrpc;
use crate::logs::LogStore;
use crate::url_test::UrlTestQueue;

/// 等待阶段变化的最长时间。
///
/// 监控线程正常时阶段变化是瞬时的；设上限只为避免监控线程异常退出时调用方无限挂起。
const WAIT_TIMEOUT: Duration = Duration::from_secs(10);

/// 与监控线程共享的运行状态。
///
/// 阶段与失败原因用 `ArcSwap` 存放，任何线程都能无锁读取；
/// 启停意图与进程句柄只由 `SingBox` 的启停方法和监控线程访问。
pub(super) struct Shared {
    /// 当前生命周期阶段。
    pub(super) phase: ArcSwap<Phase>,
    /// 最近一次异常的原因，正常运行期间为空。
    pub(super) reason: ArcSwapOption<String>,
    /// 本次启动的 API 监听地址，登记启动意图时写入。
    pub(super) api: ArcSwap<String>,
    /// 启停意图与进程句柄。
    pub(super) control: Mutex<Control>,
    /// 阶段变化与意图登记的唤醒信号。
    pub(super) wake: Condvar,
    /// gRPC 客户端，仅在内核运行期间存在。
    pub(super) grpc: ArcSwapOption<SingBoxGrpc>,
    /// 测速任务队列，随内核对象一起创建，内核启停时注入或清空客户端。
    pub(super) url_test: Arc<UrlTestQueue>,
    /// 连接数据，内核运行期间由常驻订阅把推送累积成快照。
    pub(super) connections: Arc<ConnectionStore>,
    /// 内核日志，内核运行期间由常驻读取线程把日志文件解析成块。
    pub(super) logs: Arc<LogStore>,
}

/// 启停意图与进程句柄，只由启停方法与监控线程访问。
#[derive(Default)]
pub(super) struct Control {
    /// 尚未处理的意图。
    pub(super) intent: Intent,
    /// 当前子进程，仅监控线程持有。
    pub(super) process: Option<Process>,
    /// 当前子进程的工作目录，进程异常退出时据此读取日志尾部的原始错误。
    pub(super) work: Option<PathBuf>,
    /// 实例是否已释放，监控线程看到后退出。
    pub(super) closed: bool,
}

/// 启停意图。
#[derive(Default)]
pub(super) enum Intent {
    /// 无待处理意图。
    #[default]
    Idle,
    /// 启动内核。
    Start { config: PathBuf, work: PathBuf },
    /// 停止内核。
    Stop,
}

impl Shared {
    /// 创建初始状态：未运行，没有失败原因。
    pub(super) fn new() -> Self {
        Self {
            phase: ArcSwap::from_pointee(Phase::Stopped),
            reason: ArcSwapOption::empty(),
            api: ArcSwap::from_pointee(String::new()),
            control: Mutex::new(Control::default()),
            wake: Condvar::new(),
            grpc: ArcSwapOption::empty(),
            url_test: Arc::new(UrlTestQueue::new()),
            connections: Arc::new(ConnectionStore::new()),
            logs: Arc::new(LogStore::new()),
        }
    }

    /// 当前阶段，无锁读取。
    pub(super) fn phase(&self) -> Phase {
        **self.phase.load()
    }

    /// 取内部状态，锁被污染时继续使用其内容。
    pub(super) fn lock(&self) -> MutexGuard<'_, Control> {
        self.control
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }

    /// 更新阶段与失败原因，并唤醒等待阶段变化的调用方。
    ///
    /// 持锁写入是为了与 [`wait_phase`](Self::wait_phase) 互斥：等待方在锁内检查阶段，
    /// 写入方也在锁内改阶段并唤醒，这样不会出现「等待方刚检查完、唤醒信号已发出」的丢失唤醒。
    pub(super) fn set_phase(&self, phase: Phase, reason: Option<String>) {
        let _guard = self.lock();
        self.phase.store(Arc::new(phase));
        self.reason.store(reason.map(Arc::new));
        self.wake.notify_all();
    }

    /// 等待阶段满足 `accept`，返回满足条件时的阶段。
    ///
    /// 超时说明监控线程没有响应，返回 `None` 让调用方给出明确错误而不是一直挂起。
    pub(super) fn wait_phase(&self, accept: impl Fn(Phase) -> bool) -> Option<Phase> {
        let guard = self.lock();
        let (guard, _) = self
            .wake
            .wait_timeout_while(guard, WAIT_TIMEOUT, |_| !accept(self.phase()))
            .unwrap_or_else(|error| error.into_inner());
        drop(guard);

        let phase = self.phase();
        accept(phase).then_some(phase)
    }

    /// 作废与进程绑定的数据：客户端、连接订阅都随旧进程失效。
    pub(super) fn invalidate(&self) {
        self.grpc.store(None);
        self.url_test.set_grpc(None);
        self.url_test.clear();
        self.connections.subscribe(None);
    }
}
