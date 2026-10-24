use framework_timer::{TimerResult, timer};

use crate::inner_rule;
use crate::inner_rule_sync::InnerRuleSync;
use crate::timer::{TimerName, environment};

/// 内置规则定时同步: 每 30 秒把不可用的内置规则推入同步队列。
#[timer(interval = 30_000)]
pub async fn timer_inner_rule_sync() -> TimerResult {
    match run().await {
        Ok(()) => TimerResult::success(),
        Err(error) => TimerResult::failure(error),
    }
}

/// 扫描不可用的内置规则, 推入内置规则同步调度器。
///
/// 可用要求已就绪且本地文件存在, 文件被删除的规则会被置为未就绪后重新下载。
async fn run() -> anyhow::Result<()> {
    let environment = environment()?;
    let ids = inner_rule::pending(&environment.context)
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
