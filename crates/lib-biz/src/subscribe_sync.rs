use std::sync::{Arc, LazyLock};

use anyhow::{Result, anyhow};
use lib_converter::{Platform, PlatformConfig, clash_to, pure_to};
use lib_core::{current_millis, url};
use lib_db::DbContext;
use lib_fetch::FetchClient;
use lib_pick::{KEY_TYPE, PickNodes, PickRemote, pick};
use log::{debug, error, info, warn};
use service_core::service::subscribe_service::SubscribeService;
use types_core::dto::subscribe::SubscribeRefreshDTO;

use crate::scheduler::{Scheduler, SyncTask};

/// 全局唯一的订阅同步调度器。
static INSTANCE: LazyLock<Scheduler<SubscribeTask>> = LazyLock::new(Scheduler::new);

/// 订阅同步调度器。
///
/// 使用前必须通过 [`init`](Self::init) 注入数据库上下文与异步运行时。
pub struct SubscribeSync {
    scheduler: &'static Scheduler<SubscribeTask>,
}

impl SubscribeSync {
    /// 全局唯一实例。
    pub fn instance() -> Self {
        Self {
            scheduler: &INSTANCE,
        }
    }

    /// 注入任务所需的数据库上下文与异步运行时, 重复调用会覆盖旧值。
    pub fn init(&self, context: Arc<DbContext>, runtime: tokio::runtime::Handle) {
        self.scheduler.init(context, runtime);
    }

    /// 推送单个 id, 返回是否成功入队。
    ///
    /// 已在运行或已在队列中的 id 不会重复推入。
    pub fn push(&self, id: &str) -> bool {
        self.scheduler.push(id)
    }

    /// 批量推送 id, 返回成功入队的数量。
    pub fn push_all(&self, ids: impl IntoIterator<Item = String>) -> usize {
        self.scheduler.push_all(ids)
    }

    /// 等待指定 id 的同步任务结束, 没有对应任务时直接完成。
    pub async fn wait(&self, id: &str) {
        self.scheduler.wait(id).await
    }
}

/// 订阅同步任务的定义。
struct SubscribeTask;

impl SyncTask for SubscribeTask {
    const THREAD: &'static str = "subscribe-sync";

    async fn run(context: Arc<DbContext>, id: String) {
        if let Err(error) = sync(&context, &id).await {
            error!("[订阅] [{}] 同步异常! {error:#}", id);
        }
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
