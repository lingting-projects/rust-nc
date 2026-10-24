//! 主窗口状态。
//!
//! 页面的数据都来自内核推送或本地文件，只有窗口在用户眼前时才需要跟上；
//! 隐藏到托盘后没有任何东西要展示，定时器与查询应当一起停下，
//! 否则进程会留在托盘里每秒唤醒一次、白跑一遍轮询。
//!
//! 状态用 [`ArcSwap`] 整体替换：读取不加锁也不分配，任何线程都能直接问；
//! 隐藏时轮询任务挂在 tokio 的 `watch` 上，隐藏期间不设超时、不产生唤醒，
//! 重新显示时才被叫醒并立刻刷新一次。

use std::sync::{Arc, LazyLock};
use std::time::Duration;

use arc_swap::ArcSwap;
use gpui_kit::*;
use tokio::sync::watch;

/// 全局唯一的窗口状态。
static WINDOW: LazyLock<WindowState> = LazyLock::new(WindowState::new);

/// 主窗口的状态快照。
///
/// 与 [`WindowState`] 分开：这里只放被替换的值，读取方拿到的永远是一份自洽的状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InnerWindowState {
    /// 窗口是否可见。隐藏到托盘后为 `false`
    visible: bool,
}

impl InnerWindowState {
    /// 窗口是否可见。
    pub fn visible(self) -> bool {
        self.visible
    }
}

/// 主窗口状态。
///
/// 全局唯一，通过 [`window_state`] 取用；显隐由 [`crate::tray::window`] 在
/// 真正收起或叫回窗口时写入，页面只读不写。
pub struct WindowState {
    /// 最新状态快照
    state: ArcSwap<InnerWindowState>,
    /// 状态变化通知，隐藏的轮询任务挂在它上面
    changes: watch::Sender<()>,
}

impl WindowState {
    /// 创建状态，窗口可见性默认为隐藏，由启动流程按实际显隐写入。
    fn new() -> Self {
        Self {
            state: ArcSwap::from_pointee(InnerWindowState::default()),
            // 接收端按需订阅，发送端随全局一直存活
            changes: watch::channel(()).0,
        }
    }

    /// 窗口是否可见。
    ///
    /// 直接从快照读，不加锁也不分配：每拍轮询都会问一次。
    pub fn visible(&self) -> bool {
        self.state.load().visible()
    }

    /// 更新窗口可见性，可见性变化时叫醒挂起的轮询任务。
    pub fn set_visible(&self, visible: bool) {
        if self.visible() == visible {
            return;
        }
        self.state.store(Arc::new(InnerWindowState { visible }));
        // 没有订阅者时只是写入值，不会有唤醒
        self.changes.send_replace(());
    }

    /// 等到下一次轮询时机。
    ///
    /// 可见时按 `interval` 等待；窗口隐藏时挂起到重新显示，隐藏期间不设超时，
    /// 因此不会留下任何定时器唤醒。重新显示后立即返回，调用方马上刷新一次。
    pub async fn tick(&self, cx: &AsyncApp, interval: Duration) {
        let mut changes = self.changes.subscribe();
        if !self.visible() {
            wait_visible(self, &mut changes).await;
            return;
        }

        cx.background_executor().timer(interval).await;
        // 等待期间可能被收起：挂起到重新显示，再让调用方立刻刷新一次
        wait_visible(self, &mut changes).await;
    }
}

/// 等到窗口可见。
///
/// 每次状态变化都会唤醒一次，因此循环里重新读快照即可；
/// 发送端随全局一起存活，`changed` 出错说明进程正在退出。
async fn wait_visible(state: &WindowState, changes: &mut watch::Receiver<()>) {
    while !state.visible() {
        if changes.changed().await.is_err() {
            return;
        }
    }
}

/// 全局唯一的窗口状态。
pub fn window_state() -> &'static WindowState {
    &WINDOW
}
