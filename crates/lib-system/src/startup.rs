//! 开机自启的注册与查询，按平台分发到具体实现。

use std::path::Path;

use anyhow::Result;

#[cfg(not(target_os = "windows"))]
mod unix;
#[cfg(target_os = "windows")]
mod windows;

/// 是否已注册开机自启。
pub fn is_startup() -> Result<bool> {
    #[cfg(target_os = "windows")]
    return windows::is_startup();
    #[cfg(not(target_os = "windows"))]
    return unix::is_startup();
}

/// 注册开机自启，`cache_dir` 用于存放中间文件。
pub fn enable(cache_dir: &Path) -> Result<bool> {
    #[cfg(target_os = "windows")]
    return windows::enable(cache_dir);
    #[cfg(not(target_os = "windows"))]
    return unix::enable(cache_dir);
}

/// 取消开机自启。
pub fn disable() -> Result<bool> {
    #[cfg(target_os = "windows")]
    return windows::disable();
    #[cfg(not(target_os = "windows"))]
    return unix::disable();
}
