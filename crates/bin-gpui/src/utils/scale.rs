/// 系统 DPI 基准值（100% 缩放）。
const USER_DEFAULT_SCREEN_DPI: f32 = 96.0;

/// 主屏幕的缩放因子。
///
/// GPUI 的 `WindowBounds` 是逻辑像素，而窗口实际占用物理像素，两者相差一个缩放因子。
/// `App` 与 `PlatformDisplay` 都没有公开这个值，因此这里用与 GPUI 相同的方式查询：
/// 主显示器 + `GetDpiForMonitor(MDT_EFFECTIVE_DPI)`。
///
/// 非 Windows 平台返回 1.0。
#[cfg(target_os = "windows")]
pub fn primary_scale_factor() -> f32 {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Gdi::{MONITOR_DEFAULTTOPRIMARY, MonitorFromPoint};
    use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};

    let mut dpi_x = 0;
    let mut dpi_y = 0;
    let dpi = unsafe {
        // 与 GPUI 一致：取主显示器。
        let monitor = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
        // HRESULT 小于 0 表示失败。
        if GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) < 0 {
            return 1.0;
        }
        dpi_x
    };

    if dpi == 0 {
        1.0
    } else {
        dpi as f32 / USER_DEFAULT_SCREEN_DPI
    }
}

/// 主屏幕的缩放因子。
#[cfg(not(target_os = "windows"))]
pub const fn primary_scale_factor() -> f32 {
    1.0
}
