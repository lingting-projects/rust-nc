use framework_timer::{TimerResult, timer};
use lib_core::current_millis;
use service_core::service::rule_service::RuleService;

use crate::rule_sync::RuleSync;
use crate::timer::{TimerName, environment};

/// 规则定时同步: 每 30 秒把到达刷新时间的规则推入同步队列。
#[timer(interval = 30_000)]
pub async fn timer_rule_sync() -> TimerResult {
    match run().await {
        Ok(()) => TimerResult::success(),
        Err(error) => TimerResult::failure(error),
    }
}

/// 扫描到达刷新时间的规则, 推入规则同步调度器。
async fn run() -> anyhow::Result<()> {
    let environment = environment()?;
    let millis = current_millis()?;
    let service = RuleService::new(environment.context.clone());
    let ids = service
        .list_need_refresh(millis)
        .await?
        .into_iter()
        .map(|item| item.id)
        .collect::<Vec<_>>();

    let count = RuleSync::instance().push_all(ids);
    log::debug!(
        "[规则] [{}] 推送任务数量: {count}",
        TimerName::TimerRuleSync
    );
    Ok(())
}
