use framework_timer::{TimerResult, timer};
use service_core::service::inner_rule_service::InnerRuleService;

use crate::inner_rule_sync::InnerRuleSync;
use crate::timer::{TimerName, environment};

/// 内置规则定时同步: 每 30 秒把未就绪的内置规则推入同步队列。
#[timer(interval = 30_000)]
pub async fn timer_inner_rule_sync() -> TimerResult {
    match run().await {
        Ok(()) => TimerResult::success(),
        Err(error) => TimerResult::failure(error),
    }
}

/// 扫描未就绪的内置规则, 推入内置规则同步调度器。
async fn run() -> anyhow::Result<()> {
    let environment = environment()?;
    let service = InnerRuleService::new(environment.context.clone());
    let ids = service
        .list_not_ready()
        .await?
        .into_iter()
        .map(|item| item.id)
        .collect::<Vec<_>>();

    let count = InnerRuleSync::instance().push_all(ids);
    log::debug!(
        "[内置规则] [{}] 推送任务数量: {count}",
        TimerName::TimerInnerRuleSync
    );
    Ok(())
}
