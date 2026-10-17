use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Condvar, LazyLock, Mutex, MutexGuard};

use anyhow::{Result, anyhow};
use lib_converter::{Platform, PlatformConfig, clash_to, pure_to};
use lib_core::{current_millis, url};
use lib_db::DbContext;
use lib_fetch::FetchClient;
use lib_pick::{KEY_TYPE, PickNodes, PickRemote, pick};
use log::{debug, error, info, warn};
use service_core::service::subscribe_service::SubscribeService;
use tokio::runtime::Handle;
use types_core::dto::subscribe::SubscribeRefreshDTO;

/// 并行任务数量上限。
const MAX_TASKS: usize = 5;

/// 全局唯一的订阅同步调度器。
static INSTANCE: LazyLock<SubscribeSync> = LazyLock::new(SubscribeSync::new);

/// 订阅同步调度器。
///
/// 由 [`instance`](Self::instance) 取得全局唯一实例, 使用前必须通过 [`init`](Self::init)
/// 注入数据库上下文与异步运行时。
/// 内部的主线程在推送与任务完成时被唤醒, 按并行上限的差额提交任务。
pub struct SubscribeSync {
    state: Arc<State>,
}

impl SubscribeSync {
    /// 全局唯一实例。
    pub fn instance() -> &'static Self {
        &INSTANCE
    }

    /// 注入任务所需的数据库上下文与异步运行时, 重复调用会覆盖旧值。
    pub fn init(&self, context: Arc<DbContext>, runtime: Handle) {
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
    pub fn push(&self, id: &str) -> bool {
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
    pub fn push_all(&self, ids: impl IntoIterator<Item = String>) -> usize {
        ids.into_iter().filter(|id| self.push(id)).count()
    }

    /// 等待指定 id 的同步任务结束, 没有对应任务时直接完成。
    pub async fn wait(&self, id: &str) {
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

    /// 创建调度器并启动主线程。
    fn new() -> Self {
        let state = Arc::new(State::default());
        let worker = state.clone();
        std::thread::Builder::new()
            .name("subscribe-sync".to_string())
            .spawn(move || dispatch(worker))
            .expect("订阅同步线程启动失败");

        Self { state }
    }
}

/// 主线程: 在推送与任务完成时被唤醒, 按并行上限的差额提交任务。
fn dispatch(state: Arc<State>) {
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
            run(&context, &id).await;

            let slot = worker.lock().running.remove(&id);
            if let Some(slot) = slot {
                slot.finish();
            }
            // 运行数量减少, 唤醒主线程补齐任务
            worker.wake.notify_all();
        });
    }
}

/// 同步单个订阅。
///
/// 拉取失败、返回值无效或没有节点时输出日志并结束, 不覆盖已保存的数据。
async fn run(context: &Arc<DbContext>, id: &str) {
    if let Err(error) = sync(context, id).await {
        error!("[订阅] [{}] 同步异常! {error:#}", id);
    }
}

/// 同步流程: 拉取远程数据、提取节点、转换配置, 最后保存结果。
async fn sync(context: &Arc<DbContext>, id: &str) -> Result<()> {
    let service = SubscribeService::new(context.clone());
    let Some(subscribe) = service.find_refresh(id).await? else {
        warn!("[订阅] [{}] 不存在, 结束同步", id);
        return Ok(());
    };

    info!("[订阅] [{}] 开始同步", subscribe.name);
    let source = match source(&subscribe).await {
        Ok(source) => source,
        Err(error) => {
            error!(
                "[订阅] [{}] 获取数据失败, 结束同步! {error:#}",
                subscribe.name
            );
            return Ok(());
        }
    };

    if source.remote.nodes.is_empty() {
        error!("[订阅] [{}] 未解析到节点, 结束同步", subscribe.name);
        return Ok(());
    }

    // 远端内容未变更时只刷新刷新时间, 避免重复写入相同的节点
    if let Some(body) = &source.body
        && *body == subscribe.content
    {
        info!("[订阅] [{}] 内容未变更, 仅更新刷新时间", subscribe.name);
        service.touch_refresh(id, current_millis()?).await?;
        return Ok(());
    }

    convert(&source.remote.nodes)?;
    let nodes = source.remote.nodes.to_json()?;
    debug!(
        "[订阅] [{}] 保存同步结果, 节点数量: {}",
        subscribe.name,
        source.remote.nodes.len()
    );
    service
        .save_refresh(
            id,
            source.body.as_deref(),
            &nodes,
            current_millis()?,
            source.remote.download.unwrap_or(0),
            source.remote.upload.unwrap_or(0),
            source.remote.max.unwrap_or(0),
            source.remote.expire.unwrap_or(0),
        )
        .await?;
    info!("[订阅] [{}] 同步完成", subscribe.name);
    Ok(())
}

/// 订阅数据来源。
struct Source {
    /// 远端订阅返回的完整内容, 本地订阅为 `None`
    body: Option<String>,
    /// 提取出的节点与流量信息
    remote: PickRemote,
}

/// 获取订阅数据: 远端订阅拉取远程内容, 本地订阅解析已保存的内容。
async fn source(subscribe: &SubscribeRefreshDTO) -> Result<Source> {
    if subscribe.url.is_empty() {
        debug!("[订阅] [{}] 本地订阅, 解析已保存的内容", subscribe.name);
        return Ok(Source {
            body: None,
            remote: pick(&subscribe.content, None)?,
        });
    }

    let url = url::fast(&subscribe.url);
    debug!("[订阅] [{}] 拉取远程数据: {}", subscribe.name, url);
    let response = FetchClient::new().fetch(&url).await?;
    if !response.is_success() {
        return Err(anyhow!("远程返回异常! {}", response.status));
    }

    let body = response.string();
    let info = response.subscription_userinfo().cloned();
    debug!(
        "[订阅] [{}] 获取到远程数据, 长度: {}",
        subscribe.name,
        body.len()
    );
    Ok(Source {
        body: Some(body.clone()),
        remote: pick(&body, info.as_deref())?,
    })
}

/// 转换订阅节点为内核配置。
///
/// todo: 接入配置功能后改用真实的内核设置与规则, 并把转换结果保存到配置。
fn convert(nodes: &PickNodes) -> Result<()> {
    let platform = Platform::SingBox;
    let config = PlatformConfig::default();
    let converted = if is_clash(nodes) {
        clash_to(platform, nodes.clone(), config)
    } else {
        pure_to(platform, nodes.clone(), config)
    }?;

    debug!("[订阅] 转换配置完成, 长度: {}", converted.len());
    Ok(())
}

/// 由节点识别订阅来源, 依据协议所在的字段名。
fn is_clash(nodes: &PickNodes) -> bool {
    nodes
        .nodes
        .first()
        .is_some_and(|node| node.protocol_key == KEY_TYPE)
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

/// 任务完成信号, 供 [`SubscribeSync::wait`] 等待。
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
