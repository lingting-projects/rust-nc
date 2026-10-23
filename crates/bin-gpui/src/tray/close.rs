//! 关闭按钮的语义。
//!
//! 自绘标题栏的关闭按钮走系统 `HTCLOSE → WM_CLOSE`，因此用 GPUI 的
//! `on_window_should_close` 就能拦住：返回 `false` 时窗口不关闭。
//! 隐藏到托盘时正是拦在这里，把「关闭」变成「隐藏」。

use gpui_kit::*;

use super::{CloseAction, close_action, window};

/// 在主窗口上安装关闭处理。
///
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
///
/// 回调本身已经运行在窗口上下文里，隐藏必须用 [`window::hide_now`]：
/// 再对同一个窗口发起一次更新会重复借用 `App`。
fn allow_close(window: &mut Window, _: &mut App) -> bool {
    match close_action() {
        // 拦下关闭后窗口只是隐藏，进程与内核继续运行
        CloseAction::Hide => {
            window::hide_now(window);
            false
        }
        CloseAction::Exit => true,
    }
}
