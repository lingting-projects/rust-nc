use std::sync::{Arc, LazyLock};

use anyhow::{Result, anyhow};
use lib_core::{current_millis, url};
use lib_db::DbContext;
use lib_fetch::FetchClient;
use log::{debug, error, info, warn};
use service_core::service::rule_service::RuleService;
use types_core::dto::rule::RuleRefreshDTO;
use types_core::event::RuleChanged;

use crate::config_sync::ConfigSync;
use crate::rule_file;
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
    pub fn push(&self, id: i64) -> bool {
        self.scheduler.push(id)
    }

    /// 批量推送 id, 返回成功入队的数量。
    pub fn push_all(&self, ids: impl IntoIterator<Item = i64>) -> usize {
        self.scheduler.push_all(ids)
    }
    /// 正在运行或排队中的同步任务 id。
    ///
    /// 界面据此判断「刷新中」，只读内存状态，不触发 IO。
    pub fn pending(&self) -> Vec<i64> {
        self.scheduler.pending()
    }

    /// 等待指定 id 的同步任务结束, 没有对应任务时直接完成。
    pub async fn wait(&self, id: i64) {
        self.scheduler.wait(id).await
    }
}

/// 规则同步任务的定义。
struct RuleTask;

impl SyncTask for RuleTask {
    const THREAD: &'static str = "rule-sync";

    async fn run(context: Arc<DbContext>, id: i64) {
        if let Err(error) = sync(&context, id).await {
            error!("[规则] [{}] 同步异常! {error:#}", id);
        }
    }

    fn started() {
        // 定时与依赖推送的任务直接入队, 页面靠这个事件把「刷新中」显示出来
        lib_core::bus::notify::<RuleChanged>();
    }

    fn finished() {
        // 刷新时间与「刷新中」都由同步任务改写, 结束即通知页面重读
        lib_core::bus::notify::<RuleChanged>();
    }
}

/// 同步流程: 取得规则内容、统计各类型数量, 最后保存结果。
async fn sync(context: &Arc<DbContext>, id: i64) -> Result<()> {
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

    let files = match rule_file::write(id, &source.content).await {
        Ok(files) => files,
        Err(error) => {
            // 落盘失败时不更新同步结果, 保留上一次的标识与规则文件, 下一次同步重试
            error!(
                "[规则] [{}] 写入规则文件失败, 结束同步! {error:#}",
                rule.name
            );
            return Err(error);
        }
    };
    if files.total == 0 {
        // 无可用规则时不落盘, 规则目录保持上一次的可用内容
        warn!("[规则] [{}] 未解析到规则, 结束同步", rule.name);
        return Ok(());
    }

    debug!(
        "[规则] [{}] 保存同步结果, 规则数量: {}",
        rule.name, files.total
    );
    service
        .save_refresh(
            id,
            source.body.as_deref(),
            current_millis()?,
            files.total,
            files.process,
            files.ip,
            files.other,
        )
        .await?;
    info!("[规则] [{}] 同步完成", rule.name);
    push_configs(context, id, &rule.name).await?;
    Ok(())
}

/// 推送引用了该规则的配置刷新。
///
/// 规则内容变更后, 依赖它的配置需要重新生成配置文件;
/// 配置若是内核正在运行的那一份, 由配置同步完成后通知内核重载。
async fn push_configs(context: &Arc<DbContext>, id: i64, name: &str) -> Result<()> {
    let count = ConfigSync::instance().push_by_rule(context, id).await?;
    debug!("[规则] [{name}] 推送依赖配置数量: {count}");
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
