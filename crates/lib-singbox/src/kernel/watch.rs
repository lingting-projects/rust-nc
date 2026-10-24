//! 常驻监控线程。
//!
//! 创建与结束进程都在这条线程上完成，调用方只登记意图，因此界面线程永远不会被慢 IO 挡住。

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use super::Phase;
use super::process::{failure, spawn};
use super::shared::{Intent, Shared};

/// 监控线程检查进程退出的间隔。
const POLL_INTERVAL: Duration = Duration::from_secs(1);

/// 监控线程主体：处理启停意图，并按间隔检查进程是否退出。
pub(super) fn run(shared: Arc<Shared>, bin: String) {
    loop {
        // 1. 取出登记过的意图；实例已释放时直接退出
        let intent = {
            let mut control = shared.lock();
            if control.closed {
                return;
            }
            std::mem::take(&mut control.intent)
        };
        match intent {
            Intent::Idle => {}
            Intent::Start { config, work } => start(&shared, &bin, config, work),
            Intent::Stop => stop(&shared),
        }

        // 2. 进程自行退出时更新阶段
        poll_exit(&shared);

        // 3. 等下一次检查，或被新的意图唤醒
        //
        // 用条件等待而不是普通超时等待：意图可能在进入等待前就登记完成，
        // 此时唤醒信号已经发出，普通等待会白等到超时，启动因此最多被拖慢一个轮询周期。
        let guard = shared.lock();
        let _ = shared
            .wake
            .wait_timeout_while(guard, POLL_INTERVAL, |control| {
                !control.closed && matches!(control.intent, Intent::Idle)
            })
            .unwrap_or_else(|error| error.into_inner());
    }
}

/// 创建日志文件与子进程，成功后把阶段置为启动中。
///
/// 进程创建完成只代表内核已开始加载配置，API 是否可用还要由调用方探测，
/// 因此这里不置为运行中。
fn start(shared: &Arc<Shared>, bin: &str, config: PathBuf, work: PathBuf) {
    match spawn(bin, &config, &work) {
        Ok(process) => {
            {
                let mut control = shared.lock();
                control.process = Some(process);
                control.work = Some(work.clone());
            }

            // 上一轮连接、测速任务与日志都属于旧进程
            shared.invalidate();
            shared.connections.reset();
            // 内核日志按启动重写, 读取线程改为跟随本次启动的日志文件
            shared
                .logs
                .subscribe(Some(work.join(lib_core::constants::KERNEL_LOG_FILE)));

            log::info!("[内核] 进程已创建, 阶段转为启动中");
            shared.set_phase(Phase::Starting, None);
        }
        Err(error) => {
            let reason = format!("{error:#}");
            log::error!("[内核] 启动失败: {reason}");
            shared.set_phase(Phase::Stopped, Some(reason));
        }
    }
}

/// 结束子进程并把阶段置为未运行。
fn stop(shared: &Arc<Shared>) {
    let process = {
        let mut control = shared.lock();
        control.work = None;
        control.process.take()
    };

    if let Some(mut process) = process
        && let Err(error) = process.kill()
    {
        // 进程句柄被丢弃时还会再尝试一次结束，因此这里只记录原因
        log::error!("singbox结束进程异常! {error}");
    }

    // 无论进程是否成功结束, 旧连接都不再可用
    shared.invalidate();
    shared.connections.reset();
    // 日志同样随内核一起作废, 读取线程保留并等待下一次启动
    shared.logs.subscribe(None);
    shared.set_phase(Phase::Stopped, None);
}

/// 检查进程是否已退出，退出时记录失败原因。
fn poll_exit(shared: &Arc<Shared>) {
    // 进程创建后阶段先为启动中, API 探测通过才转为运行中, 两种阶段都要检查退出
    if !matches!(shared.phase(), Phase::Starting | Phase::Running) {
        return;
    }

    let ended = {
        let mut control = shared.lock();
        let Some(process) = control.process.as_mut() else {
            return;
        };

        match process.status() {
            Ok(Some(status)) => {
                control.process = None;
                let work = control.work.take();
                Some((status, work))
            }
            Ok(None) => None,
            Err(error) => {
                log::warn!("[内核] 查询进程状态失败: {error:#}");
                None
            }
        }
    };

    let Some((status, work)) = ended else {
        return;
    };

    // 读日志尾部拼出可读的失败原因, 这是慢 IO, 但发生在监控线程上
    let reason = match super::command::check_status(status) {
        Ok(()) => None,
        Err(error) => {
            log::error!("singbox执行异常! {error}");
            Some(match work {
                Some(work) => failure(&work, error.to_string()),
                None => error.to_string(),
            })
        }
    };

    shared.invalidate();
    shared.connections.reset();
    shared.set_phase(Phase::Stopped, reason);
}
