use std::sync::{Arc, LazyLock};

use anyhow::{Result, anyhow};
use lib_core::{current_millis, url};
use lib_db::DbContext;
use lib_fetch::FetchClient;
use log::{debug, error, info, warn};
use service_core::service::rule_service::RuleService;
use types_core::dto::rule::RuleRefreshDTO;

use crate::scheduler::{Scheduler, SyncTask};

/// 全局唯一的规则同步调度器。
static INSTANCE: LazyLock<Scheduler<RuleTask>> = LazyLock::new(Scheduler::new);

/// 规则同步调度器。
///
/// 使用前必须通过 [`init`](Self::init) 注入数据库上下文与异步运行时。
pub struct RuleSync {
    scheduler: &'static Scheduler<RuleTask>,
}

impl RuleSync {
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

/// 规则同步任务的定义。
struct RuleTask;

impl SyncTask for RuleTask {
    const THREAD: &'static str = "rule-sync";

    async fn run(context: Arc<DbContext>, id: String) {
        if let Err(error) = sync(&context, &id).await {
            error!("[规则] [{}] 同步异常! {error:#}", id);
        }
    }
}

/// 同步流程: 取得规则内容、统计各类型数量, 最后保存结果。
async fn sync(context: &Arc<DbContext>, id: &str) -> Result<()> {
    let service = RuleService::new(context.clone());
    let Some(rule) = service.find_refresh(id).await? else {
        warn!("[规则] [{}] 不存在, 结束同步", id);
        return Ok(());
    };

    info!("[规则] [{}] 开始同步", rule.name);
    let source = match source(&rule).await {
        Ok(source) => source,
        Err(error) => {
            error!("[规则] [{}] 获取数据失败, 结束同步! {error:#}", rule.name);
            return Ok(());
        }
    };

    // 远端内容未变更时只刷新刷新时间, 避免重复写入相同的统计
    if let Some(body) = &source.body
        && *body == rule.content
    {
        info!("[规则] [{}] 内容未变更, 仅更新刷新时间", rule.name);
        service.touch_refresh(id, current_millis()?).await?;
        return Ok(());
    }

    let counts = count(&source.content);
    if counts.total == 0 {
        warn!("[规则] [{}] 未解析到规则, 结束同步", rule.name);
        return Ok(());
    }

    debug!(
        "[规则] [{}] 保存同步结果, 规则数量: {}",
        rule.name, counts.total
    );
    service
        .save_refresh(
            id,
            source.body.as_deref(),
            current_millis()?,
            counts.total,
            counts.process,
            counts.ip,
            counts.other,
        )
        .await?;
    info!("[规则] [{}] 同步完成", rule.name);
    Ok(())
}

/// 规则数据来源。
struct Source {
    /// 远端规则返回的完整内容, 本地规则为 `None`
    body: Option<String>,
    /// 待统计的规则文本
    content: String,
}

/// 获取规则内容: 远端规则拉取远程内容, 本地规则使用已保存的内容。
async fn source(rule: &RuleRefreshDTO) -> Result<Source> {
    if rule.url.is_empty() {
        debug!("[规则] [{}] 本地规则, 统计已保存的内容", rule.name);
        return Ok(Source {
            body: None,
            content: rule.content.clone(),
        });
    }

    let url = url::fast(&rule.url);
    debug!("[规则] [{}] 拉取远程数据: {}", rule.name, url);
    let response = FetchClient::new().fetch(&url).await?;
    if !response.is_success() {
        return Err(anyhow!("远程返回异常! {}", response.status));
    }

    let body = response.string();
    debug!(
        "[规则] [{}] 获取到远程数据, 长度: {}",
        rule.name,
        body.len()
    );
    Ok(Source {
        content: body.clone(),
        body: Some(body),
    })
}

/// 规则数量统计, 按 sing-box 规则集的类型归类。
#[derive(Debug, Default, PartialEq, Eq)]
struct Counts {
    /// 可用规则总数
    total: u64,
    /// IP 规则数量
    ip: u64,
    /// 进程规则数量
    process: u64,
    /// 其他规则数量
    other: u64,
}

/// 统计规则文本中的条目数量。
///
/// 每行一条 `类型,值`, 空行与 `#` 开头的注释行跳过;
/// `ip-cidr` 开头归为 IP 规则, `process-name` 归为进程规则, 其余归为其他规则。
fn count(content: &str) -> Counts {
    let mut counts = Counts::default();

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((kind, value)) = line.split_once(',') else {
            continue;
        };
        let (kind, value) = (kind.trim(), value.trim());
        if kind.is_empty() || value.is_empty() {
            continue;
        }

        counts.total += 1;
        let kind = kind.to_lowercase();
        if kind.starts_with("ip-cidr") {
            counts.ip += 1;
        } else if kind == "process-name" {
            counts.process += 1;
        } else {
            counts.other += 1;
        }
    }

    counts
}
