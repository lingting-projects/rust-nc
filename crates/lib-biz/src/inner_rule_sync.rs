use std::path::Path;
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use lib_db::DbContext;
use lib_fetch::{DEFAULT_THREADS, FetchClient};
use log::{debug, error, info, warn};
use service_core::service::inner_rule_service::InnerRuleService;
use types_core::entity::inner_rule::InnerRule;

use crate::file;
use crate::inner_rule_file;
use crate::scheduler::{Scheduler, SyncTask};

/// 全局唯一的内置规则同步调度器。
static INSTANCE: LazyLock<Scheduler<InnerRuleTask>> = LazyLock::new(Scheduler::new);

/// 内置规则同步轮次: 等待内核启动时每隔这么久检查一次是否就绪。
pub const WAIT_STEP: Duration = Duration::from_secs(3);

/// 内核启动前等待内置规则的轮次上限。
///
/// 每轮结束后重新检查就绪状态, 超过该轮次说明网络持续异常, 直接失败,
/// 避免启动动作被无限挂起。
pub const WAIT_ROUNDS: usize = 20;

/// 内置规则同步调度器。
///
/// 使用前必须通过 [`init`](Self::init) 注入数据库上下文与异步运行时。
pub struct InnerRuleSync {
    scheduler: &'static Scheduler<InnerRuleTask>,
}

impl InnerRuleSync {
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
    pub fn push(&self, id: i64) -> bool {
        self.scheduler.push(id)
    }

    /// 批量推送 id, 返回成功入队的数量。
    pub fn push_all(&self, ids: impl IntoIterator<Item = i64>) -> usize {
        self.scheduler.push_all(ids)
    }

    /// 等待指定 id 的同步任务结束, 没有对应任务时直接完成。
    pub async fn wait(&self, id: i64) {
        self.scheduler.wait(id).await
    }
}

/// 内置规则同步任务的定义。
struct InnerRuleTask;

impl SyncTask for InnerRuleTask {
    const THREAD: &'static str = "inner-rule-sync";

    async fn run(context: Arc<DbContext>, id: i64) {
        if let Err(error) = sync(&context, id).await {
            error!("[内置规则] [{id}] 同步异常! {error:#}");
        }
    }
}

/// 同步单条内置规则: 检查 etag, 需要时下载到本地。
async fn sync(context: &Arc<DbContext>, id: i64) -> Result<()> {
    let service = InnerRuleService::new(context.clone());
    let Some(rule) = service.find(id).await? else {
        warn!("[内置规则] [{id}] 不存在, 结束同步");
        return Ok(());
    };

    let path = inner_rule_file::inner_rule_file(&rule.name)?;
    // etag 未变且文件仍在时无需下载
    if rule.ready && path.exists() && same_etag(&rule).await {
        debug!("[内置规则] [{}] 内容未变更, 仅确认就绪", rule.name);
        return Ok(());
    }

    info!("[内置规则] [{}] 开始下载: {}", rule.name, rule.url);
    // 先下到临时文件再原子替换, 内核随时可能读取, 不能让它看到写了一半的规则集
    let temp = file::temp_path(&inner_rule_file::inner_rule_temp_dir()?)?;
    match download(&rule, &temp).await {
        Ok(etag) => {
            file::rename(&temp, &path)?;
            service.ready(id, &etag).await?;
            info!("[内置规则] [{}] 下载完成", rule.name);
            Ok(())
        }
        Err(error) => {
            // 下载失败不能继续引用旧文件, 否则内核可能读到过期内容
            file::remove_file(&temp)?;
            service.not_ready(id).await?;
            Err(error)
        }
    }
}

/// 判断远端内容是否与本地一致。
///
/// 加速镜像会给同一份文件打上自己的 etag, 与原始地址的 etag 不同,
/// 因此按下载优先顺序取第一个能用的 etag 比对: 镜像优先, 镜像不可用时看原始地址。
/// 两个来源都取不到 etag 时按「已变更」处理, 宁可多下载一次也不使用过期内容。
async fn same_etag(rule: &InnerRule) -> bool {
    if rule.etag.is_empty() {
        return false;
    }

    let client = FetchClient::new();
    let mirror = inner_rule_file::inner_rule_url(&rule.url);
    let urls = match mirror == rule.url {
        true => vec![rule.url.clone()],
        false => vec![mirror, rule.url.clone()],
    };

    for url in urls {
        let etag = client.probe(&url).await.etag;
        if !etag.is_empty() {
            return etag == rule.etag;
        }
    }
    false
}

/// 下载规则集到 `path`, 返回本次可用的 etag。
///
/// 按来源回退: 先试加速镜像的分片下载, 镜像不支持分片或分片失败时回到原始地址,
/// 原始地址也失败时再用镜像整体下载。
async fn download(rule: &InnerRule, path: &Path) -> Result<String> {
    let client = FetchClient::new();
    let mirror = inner_rule_file::inner_rule_url(&rule.url);

    if mirror != rule.url {
        match download_chunked(&client, &mirror, path).await {
            Ok(Some(etag)) => return Ok(etag),
            Ok(None) => debug!("[内置规则] [{}] 镜像不支持分片, 改用原始地址", rule.name),
            Err(error) => {
                warn!(
                    "[内置规则] [{}] 镜像分片下载失败, 改用原始地址: {error:#}",
                    rule.name
                );
            }
        }
    }

    match download_any(&client, &rule.url, path).await {
        Ok(etag) => Ok(etag),
        Err(error) => {
            // 原始地址不可用时退回镜像整体下载, 部分镜像只支持整体 GET
            warn!(
                "[内置规则] [{}] 原始地址下载失败, 改用镜像: {error:#}",
                rule.name
            );
            let (_, probe) = client.download_single_etag(&mirror, path).await?;
            Ok(probe.etag)
        }
    }
}

/// 仅在支持分片时分片下载, 否则返回 `None` 交给调用方换来源。
async fn download_chunked(client: &FetchClient, url: &str, path: &Path) -> Result<Option<String>> {
    let probe = client.probe(url).await;
    if !probe.chunkable(DEFAULT_THREADS) {
        return Ok(None);
    }

    let outcome = client
        .download_probed(url, path, DEFAULT_THREADS, probe.clone())
        .await?;
    debug!(
        "[内置规则] 分片下载完成: {url}, 字节数: {}",
        outcome.written
    );
    Ok(Some(probe.etag))
}

/// 按远端能力下载: 支持分片时并行, 否则顺序下载。
async fn download_any(client: &FetchClient, url: &str, path: &Path) -> Result<String> {
    let probe = client.probe(url).await;
    if !probe.length.is_some_and(|length| length > 0) {
        bail!("无法确认文件大小: {url}");
    }

    let outcome = client
        .download_probed(url, path, DEFAULT_THREADS, probe.clone())
        .await?;
    debug!(
        "[内置规则] 下载完成: {url}, 字节数: {}, 分片: {}",
        outcome.written, outcome.chunked
    );
    Ok(probe.etag)
}

/// 等待全部内置规则就绪，供内核启动前调用。
///
/// 未就绪时直接同步下载并等待, 不经过调度器: 调用方必须拿到确定结果才能启动内核,
/// 走调度器会把同步结果变成后台任务, 等待反而依赖调度器的初始化状态。
/// 最多尝试 [`WAIT_ROUNDS`] 轮, 每轮间隔 [`WAIT_STEP`], 避免网络持续异常时无限挂起。
pub async fn wait_ready(context: &Arc<DbContext>) -> Result<()> {
    let service = InnerRuleService::new(context.clone());

    for round in 1..=WAIT_ROUNDS {
        let pending = service.list_not_ready().await?;
        if pending.is_empty() {
            return Ok(());
        }

        let names = pending
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        debug!("[内置规则] 第 {round} 轮等待就绪: {names}");
        // 单条失败不直接中断, 全轮结束后统一检查, 让可用的规则先落盘
        for item in &pending {
            if let Err(error) = sync(context, item.id).await {
                warn!(
                    "[内置规则] [{}] 第 {round} 轮下载失败: {error:#}",
                    item.name
                );
            }
        }

        if service.list_not_ready().await?.is_empty() {
            return Ok(());
        }
        tokio::time::sleep(WAIT_STEP).await;
    }

    let pending = service.list_not_ready().await?;
    let names = pending
        .iter()
        .map(|item| item.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    Err(anyhow!("内置规则未就绪: {names}"))
}
