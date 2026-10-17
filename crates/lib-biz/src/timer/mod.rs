//! 定时同步任务。
//!
//! 三类数据各自有一个扫描任务, 由 `framework-timer` 按固定间隔唤醒:
//! 取出到达刷新时间的数据 id, 推入对应的同步调度器, 由调度器控制并行上限。
//!
//! 使用前必须通过 [`init`] 注入数据库上下文与异步运行时, 再由 [`start`] 启动运行时。

mod config;
mod rule;
mod subscribe;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

use anyhow::{Result, anyhow};
use framework_timer::TimerOption;
use lib_db::DbContext;
use tokio::runtime::Handle;

pub use config::timer_config_sync;
pub use rule::timer_rule_sync;
pub use subscribe::timer_subscribe_sync;

/// 由 `build.rs` 扫描 `#[timer]` 任务生成的名称枚举。
///
/// 任务在日志中记录自身名称时使用该枚举, 而不是手写字符串字面量,
/// 任务函数改名后编译期即可发现。
pub use crate::timer_name::TimerName;

/// 定时任务执行所需的环境。
struct Environment {
    context: Arc<DbContext>,
    runtime: Handle,
}

static ENVIRONMENT: OnceLock<Environment> = OnceLock::new();

/// 定时任务运行时是否已启动, `framework_timer::start` 只能调用一次。
static STARTED: AtomicBool = AtomicBool::new(false);

/// 注入定时任务所需的数据库上下文与异步运行时, 重复调用会被忽略。
pub fn init(context: Arc<DbContext>, runtime: Handle) {
    let _ = ENVIRONMENT.set(Environment { context, runtime });
}

/// 启动定时任务运行时, 重复调用直接返回成功。
///
/// 运行时会派发任务到注入的异步运行时, 因此这里先进入该运行时的上下文。
pub fn start() -> Result<()> {
    let environment = environment()?;
    if STARTED.swap(true, Ordering::SeqCst) {
        return Ok(());
    }

    let _guard = environment.runtime.enter();
    framework_timer::start(
        None,
        TimerOption {
            max_concurrent_tasks: None,
            retry_immediately_on_panic: false,
        },
    );
    Ok(())
}

/// 取定时任务执行环境, 未初始化时返回错误。
fn environment() -> Result<&'static Environment> {
    ENVIRONMENT.get().ok_or_else(|| anyhow!("定时同步未初始化"))
}
