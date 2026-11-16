//! 常驻监控线程。
//!
//! 创建与结束进程都在这一条线程上完成，调用方只登记意图，因此界面线程永远不会被慢 IO 挡住；
//! 内核重载也走这里：写重载标记、按秒等内核的重载完成标记，期间阶段为 [`Phase::Reloading`]。

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::Phase;
use super::process::{failure, remove_marker, spawn};
use super::shared::{Intent, Shared};

/// 监控线程检查进程退出与重载完成的间隔。
const POLL_INTERVAL: Duration = Duration::from_secs(1);

/// 等待内核完成重载的最长时间。
///
/// 正常情况下内核在 1~2 秒内就重建完实例并创建完成标记；设上限只为避免内核没能
/// 创建标记时界面永远停在重载中。
const RELOAD_TIMEOUT: Duration = Duration::from_secs(15);

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
            Intent::Reload => reload(&shared),
        }

        // 2. 进程自行退出时更新阶段
        poll_exit(&shared);

        // 3. 内核完成重载后恢复运行中，期间不给客户端
        poll_reload(&shared);

        // 4. 等下一次检查，或被新的意图唤醒
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
                // 新进程不再受上一次重载的期限与待重载标记约束
                control.reload_deadline = None;
                control.reload_pending = false;
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
        control.reload_deadline = None;
        control.reload_pending = false;
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
    // 进程创建后阶段先为启动中, API 探测通过才转为运行中; 重载期间实例在重建但进程还在,
    // 这些阶段都要检查退出
    if !matches!(
        shared.phase(),
        Phase::Starting | Phase::Running | Phase::Reloading
    ) {
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

/// 通知内核重建实例：写完重载标记后由 [`poll_reload`] 等内核给出完成标记。
///
/// 重载期间不给客户端：旧实例的连接在实例关闭后仍然存活但 gRPC 服务已停止，
/// 此时建连只会得到不再返回状态的僵尸连接。
fn reload(shared: &Arc<Shared>) {
    let work = shared.lock().work.clone();
    let Some(work) = work else {
        // 进程已经不存在, 重载无处可去, 回到未运行让界面不再等待
        log::warn!("[内核] 重载时没有运行中的进程");
        shared.set_phase(Phase::Stopped, None);
        return;
    };

    // 旧实例会随重载关闭, 客户端、订阅与测速任务一并作废
    shared.invalidate();
    shared.connections.reset();

    let flag = work.join(lib_core::constants::KERNEL_RELOAD_FILE);
    let done = work.join(lib_core::constants::KERNEL_RELOAD_DONE_FILE);
    // 上一次重载的完成标记会被当成本次已完成, 写标记前先清掉
    remove_marker(&done);
    if let Err(error) = std::fs::write(&flag, []) {
        let reason = format!("创建内核重载标记失败: {}: {error}", flag.display());
        log::error!("[内核] 重载失败: {reason}");
        shared.set_phase(Phase::Running, Some(reason));
        return;
    }

    shared.lock().reload_deadline = Some(Instant::now() + RELOAD_TIMEOUT);
    log::info!("[内核] 已通知内核重载, 等待实例重建完成");
}

/// 检查内核是否已完成重载。
///
/// 重载中不提供客户端：内核重建完成后会创建重载完成标记，此时才回到运行中，
/// 调用方随后按新实例重建客户端。
fn poll_reload(shared: &Arc<Shared>) {
    if shared.phase() != Phase::Reloading {
        return;
    }

    let (work, deadline) = {
        let control = shared.lock();
        (control.work.clone(), control.reload_deadline)
    };

    if let Some(work) = work {
        let done = work.join(lib_core::constants::KERNEL_RELOAD_DONE_FILE);
        if done.exists() {
            // 完成标记用完即删, 避免下一次重载被当成本次已完成
            remove_marker(&done);
            // 重载期间又来了新的配置: 立刻按最新配置再重建一次, 阶段保持重载中
            if std::mem::take(&mut shared.lock().reload_pending) {
                log::info!("[内核] 重载期间又收到新配置, 立刻再重载一次");
                reload(shared);
                return;
            }
            log::info!("[内核] 重载完成, 阶段恢复运行中");
            finish_reload(shared);
            return;
        }
    }

    // 内核没能给出完成标记时不能让界面一直停在重载中; 此时实例早已重建完, 建连是安全的
    if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
        log::warn!("[内核] 等待重载完成超时, 阶段恢复运行中");
        finish_reload(shared);
    }
}

/// 结束重载：清掉期限与待重载标记并回到运行中。
fn finish_reload(shared: &Arc<Shared>) {
    {
        let mut control = shared.lock();
        control.reload_deadline = None;
        control.reload_pending = false;
    }
    shared.set_phase(Phase::Running, None);
}
