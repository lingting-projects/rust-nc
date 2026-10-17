use framework_timer::{TimerResult, timer};
use lib_core::current_millis;
use service_core::service::subscribe_service::SubscribeService;

use crate::subscribe_sync::SubscribeSync;
use crate::timer::environment;

/// 订阅定时同步: 每 60 秒把到达刷新时间的订阅推入同步队列。
#[timer(interval = 60_000)]
pub async fn timer_subscribe_sync() -> TimerResult {
    match run().await {
        Ok(()) => TimerResult::success(),
        Err(error) => TimerResult::failure(error),
    }
}

/// 扫描到达刷新时间的订阅, 推入订阅同步调度器。
async fn run() -> anyhow::Result<()> {
    let environment = environment()?;
    let millis = current_millis()?;
    let service = SubscribeService::new(environment.context.clone());
    let ids = service
        .list_need_refresh(millis)
        .await?
        .into_iter()
        .map(|item| item.id)
        .collect::<Vec<_>>();

    let count = SubscribeSync::instance().push_all(ids);
    log::debug!("[订阅] 定时同步推送任务数量: {count}");
    Ok(())
}
