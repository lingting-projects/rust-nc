use anyhow::{Context as _, Result};

use lib_core::directorys;
use lib_core::logging::{LoggingConfig, LoggingGuard, init as init_logging};

/// 初始化日志：写入应用日志目录，同时输出到控制台与聚合日志文件。
///
/// 后台同步任务（订阅、规则、配置）的失败原因只能通过日志看到，
/// 因此日志必须在数据层启动之前初始化。
/// 返回的守卫必须存活到进程退出，否则日志写入线程会被提前释放、尾部日志丢失。
pub fn init() -> Result<LoggingGuard> {
    let directory = directorys().context("获取应用目录失败")?;

    let mut config = LoggingConfig::new();
    config.directory = Some(directory.logs.clone());
    config.combined = true;

    let guard = init_logging(&config).context("初始化日志失败")?;
    log::info!("日志初始化完成: {}", directory.logs.display());
    Ok(guard)
}
