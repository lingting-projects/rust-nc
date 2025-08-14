use library_core::core::AnyResult;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(not(target_os = "windows"))]
mod unix;

pub fn is_startup() -> AnyResult<bool> {
    #[cfg(target_os = "windows")]
    return windows::is_startup();
    #[cfg(not(target_os = "windows"))]
    return unix::is_startup();
}

pub fn enable() -> AnyResult<bool> {
    #[cfg(target_os = "windows")]
    return windows::enable();
    #[cfg(not(target_os = "windows"))]
    return unix::enable();
}
pub fn disable() -> AnyResult<bool> {
    #[cfg(target_os = "windows")]
    return windows::disable();
    #[cfg(not(target_os = "windows"))]
    return unix::disable();
}

