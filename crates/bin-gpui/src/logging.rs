use std::panic;
use std::sync::OnceLock;

use anyhow::{Context as _, Result};

use lib_core::directorys;
use lib_core::logging::{LoggingConfig, LoggingGuard, init as init_logging};

/// 日志守卫。
///
/// panic 可能发生在任意线程，hook 拿不到调用方的栈上变量，因此这里存成全局；
/// 不放进 `main` 的局部变量，就不会因为作用域结束而提前释放写入线程。
static GUARD: OnceLock<LoggingGuard> = OnceLock::new();

/// 初始化日志：写入应用日志目录，同时输出到控制台与聚合日志文件。
///
/// 后台同步任务（订阅、规则、配置）的失败原因只能通过日志看到，
/// 因此日志必须在数据层启动之前初始化。
pub fn init() -> Result<()> {
    let directory = directorys().context("获取应用目录失败")?;

    let mut config = LoggingConfig::new();
    config.directory = Some(directory.logs.clone());
    config.combined = true;

    let guard = init_logging(&config).context("初始化日志失败")?;
    // 已经初始化过一次时保留原有的守卫，写入线程不变
    let _ = GUARD.set(guard);
    log::info!("日志初始化完成: {}", directory.logs.display());
    Ok(())
}

/// 把 panic 写进日志。
///
/// release 构建会隐藏控制台窗口，Rust 默认的 panic 输出落在不可见的 stderr 上，
/// 界面崩溃后日志里只剩一条没有原因的断档。这里改成写日志，并在进程被中止前主动落盘：
/// panic 走到 abort 时析构不会运行，异步日志写入线程来不及收尾，最后一条日志会丢失。
pub fn install_panic_hook() {
    let current = panic::take_hook();

    panic::set_hook(Box::new(move |info| {
        log::error!("界面崩溃: {info}");
        flush();
        // 保留默认输出，debug 构建下直接跑终端仍能看到完整 backtrace
        current(info);
    }));
}

/// 把缓冲中的日志推到磁盘。
///
/// 正常退出与 panic 中止前都要调用：日志是异步写的，不主动落盘时最后一批会丢。
/// 日志未初始化时什么都不做。
pub fn flush() {
    if let Some(guard) = GUARD.get() {
        guard.flush();
    }
}
