use std::fs;
use std::future::pending;

use anyhow::{Context as _, Result};
use gpui_kit::*;
use lib_core::directorys;
use rust_single::{Single, SingleBuild};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

/// 单进程锁文件名，位于应用缓存目录。
const LOCK_FILE: &str = "single.lock";

/// 单进程守卫。
///
/// 持有独占锁直到进程退出：拿到锁即表示当前进程是唯一实例。
/// 后续启动的进程通过 IPC 发来唤醒信号，由界面线程把主窗口前置。
pub struct Guard {
    /// 锁本体。锁文件的独占句柄由它持有，丢弃即释放。
    single: Option<Single>,
    /// 唤醒信号接收端，由界面线程消费。
    wake: UnboundedReceiver<()>,
}

impl Guard {
    /// 获取独占锁。
    ///
    /// 已有实例在运行时先唤醒它，再返回错误，调用方据此直接退出。
    pub fn create() -> Result<Self> {
        let cache = directorys().context("获取应用目录失败")?.cache.clone();
        let (sender, wake) = unbounded_channel();

        let single = SingleBuild::new(cache.join(LOCK_FILE))
            .with_ipc(move |_| wake_sender(&sender))
            .build()
            .map_err(|error| anyhow::anyhow!("初始化单进程锁失败: {error}"))?;

        if single.is_single {
            return Ok(Self {
                single: Some(single),
                wake,
            });
        }

        // 不是唯一实例: 唤醒已运行的进程后退出，避免两个进程同时操作数据库与内核。
        let pid = single.pid.unwrap_or_default();
        single
            .wake("")
            .map_err(|error| anyhow::anyhow!("唤醒已运行实例失败: {error}"))?;
        anyhow::bail!("已存在运行中的实例: pid={pid}");
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        let Some(single) = self.single.take() else {
            return;
        };
        let path = single.path.clone();
        let path_info = single.path_info.clone();
        // 必须先释放独占锁再删除文件，否则本进程仍占用着锁文件。
        // 不删除会在以不同权限启动时残留文件，导致下一次获取锁失败。
        drop(single);
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(path_info);
    }
}

/// 在界面线程上监听唤醒信号。
///
/// 守卫随任务一起存活到进程退出，锁因此在整个运行期间不会被释放。
pub fn listen(cx: &mut App, guard: Guard, window: AnyWindowHandle) {
    cx.spawn(async move |cx| {
        let mut guard = guard;
        while guard.wake.recv().await.is_some() {
            focus(cx, window);
        }

        // 唤醒通道关闭后仍要持有锁，进程存活期间不允许另一个实例启动。
        let _guard = guard;
        pending::<()>().await
    })
    .detach();
}

/// 把主窗口前置到用户面前。
///
/// 复用托盘模块的显示逻辑：窗口可能已经被「隐藏到托盘」收起，
/// 只调 `activate_window` 不会让它重新出现。
fn focus(cx: &mut AsyncApp, window: AnyWindowHandle) {
    cx.update(|cx| crate::tray::show(cx, window));
}

/// 把唤醒信号转交给界面线程。
///
/// IPC 回调运行在独立线程上，不能直接接触 GPUI 的状态。
fn wake_sender(sender: &UnboundedSender<()>) {
    if sender.send(()).is_err() {
        log::error!("唤醒信号转交失败: 界面已退出");
    }
}
