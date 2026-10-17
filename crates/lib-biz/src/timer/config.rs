use framework_timer::{TimerResult, timer};
use lib_core::current_millis;
use service_core::service::config_service::ConfigService;

use crate::config_sync::ConfigSync;
use crate::timer::environment;

/// 配置定时同步: 每 60 秒把到达刷新时间的配置推入同步队列。
#[timer(interval = 60_000)]
pub async fn timer_config_sync() -> TimerResult {
    match run().await {
        Ok(()) => TimerResult::success(),
        Err(error) => TimerResult::failure(error),
    }
}

/// 扫描到达刷新时间的配置, 推入配置同步调度器。
async fn run() -> anyhow::Result<()> {
    let environment = environment()?;
    let millis = current_millis()?;
    let service = ConfigService::new(environment.context.clone());
    let ids = service
        .list_need_refresh(millis)
        .await?
        .into_iter()
        .map(|item| item.id)
        .collect::<Vec<_>>();

    let count = ConfigSync::instance().push_all(ids);
    log::debug!("[配置] 定时同步推送任务数量: {count}");
    Ok(())
}
