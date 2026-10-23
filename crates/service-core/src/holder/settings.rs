//! 设置的内存持有者。
//!
//! 后台线程在启动时与每次唤醒后把 `app_config` 的全部配置读到内存,
//! 上层读取设置不再访问数据库; 设置写入成功后唤醒线程刷新,
//! 保证之后读到的始终是最近一次写入的结果。

use std::sync::{Arc, Condvar, LazyLock, Mutex, MutexGuard};

use arc_swap::ArcSwapOption;
use lib_db::DbContext;
use tokio::runtime::Handle;
use types_core::dto::setting::{Setting, SettingKernel, SettingRun, SettingSoftware};

use super::snapshot::Snapshot;

/// 全局唯一的内存设置持有者。
static INSTANCE: LazyLock<SettingsHolder> = LazyLock::new(SettingsHolder::new);

/// 设置的内存持有者。
///
/// 读取与唤醒都直接调用本类型的关联方法, 由全局实例完成,
/// 调用方无需持有实例。使用前必须通过 [`init`](Self::init) 注入数据库上下文与异步运行时,
/// 之后所有读取都直接取内存快照, 不访问数据库。
pub struct SettingsHolder {
    state: Arc<State>,
}

impl SettingsHolder {
    /// 创建持有者并启动后台加载线程, 仅由全局静态量调用。
    fn new() -> Self {
        let state = Arc::new(State::default());
        let worker = state.clone();
        std::thread::Builder::new()
            .name("settings-holder".to_string())
            .spawn(move || dispatch(worker))
            .expect("设置加载线程启动失败");

        Self { state }
    }

    /// 全局唯一实例。
    fn instance() -> &'static Self {
        &INSTANCE
    }

    /// 注入数据库上下文与异步运行时, 并等待首次加载完成。
    ///
    /// 加载在后台线程进行, 这里阻塞到它结束: 之后读到的都是数据库中的最新配置,
    /// 而不是默认值。重复调用会覆盖旧值并重新加载。
    pub fn init(context: Arc<DbContext>, runtime: Handle) {
        Self::instance().start(context, runtime);
    }

    /// 唤醒后台线程重新加载全部设置, 并等待加载完成。
    ///
    /// 设置写入成功后调用: 返回时内存中已经是刚写入的设置,
    /// 后续的读取不会拿到写入前的旧值。
    pub async fn wake_and_wait() {
        Self::instance().refresh().await;
    }
}

impl SettingsHolder {
    /// 全部设置。
    pub fn all() -> Setting {
        Self::instance().snapshot().all()
    }

    /// 内核设置。
    pub fn kernel() -> SettingKernel {
        Self::instance().snapshot().kernel()
    }

    /// 软件设置。
    pub fn software() -> SettingSoftware {
        Self::instance().snapshot().software()
    }

    /// 运行设置。
    pub fn run() -> SettingRun {
        Self::instance().snapshot().run()
    }
}

impl SettingsHolder {
    /// 注入运行环境并等待首次加载完成。
    fn start(&self, context: Arc<DbContext>, runtime: Handle) {
        let target = {
            let mut inner = self.state.lock();
            inner.context = Some(context);
            inner.runtime = Some(runtime);
            inner.request()
        };
        self.state.wake.notify_all();
        self.state.wait(target);
    }

    /// 登记一次刷新并等待完成。
    async fn refresh(&self) {
        let target = {
            let mut inner = self.state.lock();
            let target = inner.request();
            self.state.wake.notify_all();
            target
        };

        // 等待放到阻塞线程, 避免占住调用方的异步线程
        let state = self.state.clone();
        let _ = tokio::task::spawn_blocking(move || state.wait(target)).await;
    }

    /// 当前快照, 尚未加载完成时按空配置转换, 各项退回默认值。
    fn snapshot(&self) -> Arc<Snapshot> {
        self.state.snapshot.load_full().unwrap_or_default()
    }
}

/// 后台线程: 被唤醒时重新加载全部设置。
fn dispatch(state: Arc<State>) {
    loop {
        // 取出本次加载的目标代数与运行环境后立刻放锁, 加载过程不持锁
        let (target, context, runtime) = {
            let mut inner = state.lock();
            loop {
                if inner.pending
                    && let (Some(context), Some(runtime)) =
                        (inner.context.clone(), inner.runtime.clone())
                {
                    inner.pending = false;
                    break (inner.requested, context, runtime);
                }

                inner = state
                    .wake
                    .wait(inner)
                    .unwrap_or_else(|error| error.into_inner());
            }
        };

        match runtime.block_on(Snapshot::load(&context)) {
            Ok(snapshot) => state.snapshot.store(Some(Arc::new(snapshot))),
            // 加载失败保留上一次的快照, 上层继续用旧设置而不是退回默认值
            Err(error) => log::error!("[设置] 加载失败, 继续使用上一次的设置: {error:#}"),
        }

        state.lock().loaded = target;
        state.wake.notify_all();
    }
}

/// 持有者的共享状态。
#[derive(Default)]
struct State {
    /// 最新设置快照, 尚未加载完成时为空
    snapshot: ArcSwapOption<Snapshot>,
    /// 加载条件与进度
    inner: Mutex<Inner>,
    /// 唤醒信号, 加载线程与等待方共用
    wake: Condvar,
}

impl State {
    /// 取共享状态, 锁被污染时继续使用其内容。
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|error| error.into_inner())
    }

    /// 阻塞等待目标代数加载完成。
    fn wait(&self, target: u64) {
        let mut inner = self.lock();
        while inner.loaded < target {
            inner = self
                .wake
                .wait(inner)
                .unwrap_or_else(|error| error.into_inner());
        }
    }
}

/// 加载线程的内部数据。
#[derive(Default)]
struct Inner {
    /// 加载所需的数据库上下文, 未初始化时为空
    context: Option<Arc<DbContext>>,
    /// 加载所需的异步运行时, 未初始化时为空
    runtime: Option<Handle>,
    /// 是否有待处理的唤醒
    pending: bool,
    /// 已完成的加载代数
    loaded: u64,
    /// 已请求的加载代数
    requested: u64,
}

impl Inner {
    /// 登记一次加载请求, 返回本次请求的代数。
    fn request(&mut self) -> u64 {
        self.pending = true;
        self.requested += 1;
        self.requested
    }
}
