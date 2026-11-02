//! 关闭按钮的语义。
//!
//! 自绘标题栏的关闭按钮走系统 `HTCLOSE → WM_CLOSE`，因此用 GPUI 的
//! `on_window_should_close` 就能拦住：返回 `false` 时窗口不关闭。
//!
//! 窗口可以随关闭销毁：进程、内核与托盘常驻，托盘唤回时再重建窗口。

use gpui_kit::*;

use super::{CloseAction, close_action};

/// 在主窗口上安装关闭处理。
///
/// 每次创建（含重建）窗口都要重新安装：拦截挂在窗口上。
/// 每次关闭都重新读设置，用户在设置页改完开关后不需要重启即可生效。
pub fn install(cx: &mut App, window: AnyWindowHandle) {
    let result = window.update(cx, |_, window, cx| {
        window.on_window_should_close(cx, allow_close)
    });

    if let Err(error) = result {
        log::error!("[托盘] 注册关闭处理失败: {error:#}");
    }
}

/// 是否允许这次关闭。
fn allow_close(_: &mut Window, cx: &mut App) -> bool {
    match close_action() {
        // 关闭后留在托盘：允许关闭，窗口随之销毁，进程与内核继续运行
        CloseAction::Tray => {
            log::info!("[退出] 关闭窗口, 进程留在托盘");
            true
        }
        // 关闭即退出。退出模式是「仅显式退出」，关掉窗口不再连带结束进程，
        // 必须在这里主动发起退出，否则窗口消失后应用会留在托盘里。
        CloseAction::Exit => {
            log::info!("[退出] 关闭按钮触发退出");
            cx.quit();
            true
        }
    }
}
