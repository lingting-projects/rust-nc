use std::path::Path;

use anyhow::Result;

#[cfg(not(target_os = "windows"))]
mod unix;
#[cfg(target_os = "windows")]
mod windows;

/// 用系统资源管理器打开目标路径。
///
/// 目标目录已经在某个资源管理器窗口中打开时，直接把该窗口切到前台；
/// 否则交给系统打开，Windows 11 的多标签资源管理器会复用已有窗口而不是新开窗口。
pub fn open_path(path: &Path) -> Result<()> {
    #[cfg(target_os = "windows")]
    return windows::open_path(path);
    #[cfg(not(target_os = "windows"))]
    return unix::open_path(path);
}
