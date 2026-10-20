use std::sync::{Arc, LazyLock};

use anyhow::{Result, anyhow};
use lib_core::current_millis;
use lib_db::DbContext;
use log::{debug, error, info, warn};
use service_core::service::config_service::ConfigService;
use service_core::service::rule_service::RuleService;
use service_core::service::subscribe_service::SubscribeService;
use types_core::entity::config::Config;

use crate::config_file;
use crate::scheduler::{Scheduler, SyncTask};

/// 全局唯一的配置同步调度器。
static INSTANCE: LazyLock<Scheduler<ConfigTask>> = LazyLock::new(Scheduler::new);

/// 配置同步调度器。
///
/// 负责批量同步: 把配置 id 推入调度器, 由后台任务按并行上限校验。
/// 单个配置由 [`sync`] 直接执行, 便于调用方拿到校验结果。
pub struct ConfigSync {
    scheduler: &'static Scheduler<ConfigTask>,
}

impl ConfigSync {
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
}

/// 配置同步任务的定义。
struct ConfigTask;

impl SyncTask for ConfigTask {
    const THREAD: &'static str = "config-sync";

    async fn run(context: Arc<DbContext>, id: String) {
        if let Err(error) = sync(&context, &id).await {
            error!("[配置] [{}] 同步异常! {error:#}", id);
        }
    }
}

/// 同步单个配置: 校验订阅节点与引用的规则, 通过后生成配置文件并刷新刷新时间。
pub(crate) async fn sync(context: &Arc<DbContext>, id: &str) -> Result<()> {
    let service = ConfigService::new(context.clone());
    let Some(config) = service.find(id).await? else {
        warn!("[配置] [{}] 不存在, 结束同步", id);
        return Ok(());
    };

    info!("[配置] [{}] 开始同步", config.name);
    let count = validate(context, &config).await?;
    let path = config_file::write(context, &config).await?;
    service.touch_refresh(id, current_millis()?).await?;
    info!(
        "[配置] [{}] 同步完成, 节点数量: {count}, 配置文件: {}",
        config.name,
        path.display()
    );
    Ok(())
}

/// 校验配置引用的订阅与规则, 返回订阅节点数量。
async fn validate(context: &Arc<DbContext>, config: &Config) -> Result<usize> {
    let count = nodes(context, config).await?;

    // 三类规则共用同一张表, 因此一次读取即可判断全部引用
    let rules = RuleService::new(context.clone()).list_ids().await?;
    let missing: Vec<&str> = config
        .rule_direct_ids
        .iter()
        .chain(&config.rule_proxy_ids)
        .chain(&config.rule_reject_ids)
        .map(String::as_str)
        .filter(|id| !rules.iter().any(|exist| exist == id))
        .collect();
    if !missing.is_empty() {
        return Err(anyhow!("规则不存在: {}", missing.join(", ")));
    }

    debug!(
        "[配置] [{}] 校验通过, 节点数量: {}, 规则数量: {}",
        config.name,
        count,
        config.rule_direct_ids.len() + config.rule_proxy_ids.len() + config.rule_reject_ids.len()
    );
    Ok(count)
}

/// 校验订阅已同步出节点, 返回节点数量。
async fn nodes(context: &Arc<DbContext>, config: &Config) -> Result<usize> {
    if config.subscribe_id.is_empty() {
        return Err(anyhow!("未选择订阅"));
    }

    let service = SubscribeService::new(context.clone());
    let Some(subscribe) = service.find(&config.subscribe_id).await? else {
        return Err(anyhow!("订阅不存在: {}", config.subscribe_id));
    };

    let (nodes, _) = service
        .find_nodes(&config.subscribe_id)
        .await?
        .unwrap_or_default();
    let count = match nodes.trim().is_empty() {
        true => 0,
        false => serde_json::from_str::<Vec<serde_json::Value>>(&nodes)?.len(),
    };
    if count == 0 {
        return Err(anyhow!("订阅「{}」尚未同步出节点", subscribe.name));
    }
    Ok(count)
}
