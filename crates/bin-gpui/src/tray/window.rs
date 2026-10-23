//! 主窗口的显隐。
//!
//! GPUI 只提供 `minimize_window` 与 `activate_window`，Windows 下没有隐藏窗口的接口，
//! 而「隐藏到托盘」要求窗口从任务栏消失，因此 Windows 上直接取 HWND 调 `ShowWindow`。
//! 隐藏之后 `activate_window` 不会让窗口重新出现，显示时必须先恢复可见性再前置。

use gpui_kit::*;

#[cfg(target_os = "windows")]
use raw_window_handle::{HasWindowHandle as _, RawWindowHandle};
#[cfg(target_os = "windows")]
use winapi::shared::windef::HWND;
#[cfg(target_os = "windows")]
use winapi::um::winuser::{SW_HIDE, SW_SHOWNORMAL, ShowWindow};

/// 显示并前置窗口。
pub fn show(cx: &mut App, window: AnyWindowHandle) {
    update(cx, window, show_now);
}

/// 隐藏窗口。
pub fn hide(cx: &mut App, window: AnyWindowHandle) {
    update(cx, window, |window, _| hide_now(window));
}

/// 在当前窗口上显示并前置。
///
/// 供已经在窗口上下文里的调用方使用：关闭回调里不能再对同一个窗口发起一次更新。
pub fn show_now(window: &mut Window, cx: &mut App) {
    #[cfg(target_os = "windows")]
    if let Some(hwnd) = hwnd(window) {
        unsafe { ShowWindow(hwnd, SW_SHOWNORMAL) };
    }
    // 隐藏态下 `activate_window` 只会激活一个看不见的窗口，所以顺序是先恢复再前置。
    window.activate_window();
    cx.activate(true);
}

/// 在当前窗口上隐藏。
///
/// 没有托盘的平台只能最小化：隐藏之后没有入口再把窗口叫回来。
pub fn hide_now(window: &mut Window) {
    #[cfg(target_os = "windows")]
    if let Some(hwnd) = hwnd(window) {
        unsafe { ShowWindow(hwnd, SW_HIDE) };
    }
    #[cfg(not(target_os = "windows"))]
    window.minimize_window();
}

/// 在窗口的上下文里执行 `f`，窗口已关闭时只记录日志。
fn update(cx: &mut App, window: AnyWindowHandle, f: impl FnOnce(&mut Window, &mut App)) {
    if let Err(error) = window.update(cx, |_, window, cx| f(window, cx)) {
        log::error!("[托盘] 操作主窗口失败: {error:#}");
    }
}

/// 窗口的 HWND。
#[cfg(target_os = "windows")]
fn hwnd(window: &mut Window) -> Option<HWND> {
    let handle = window.window_handle().ok()?;
    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Some(handle.hwnd.get() as HWND),
        _ => {
            log::error!("[托盘] 窗口句柄不是 Win32 句柄");
            None
        }
    }
}
