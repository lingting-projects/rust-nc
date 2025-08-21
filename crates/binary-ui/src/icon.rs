use library_core::core::AnyResult;

#[cfg(target_os = "windows")]
pub static path: &'static str = "icons/256x256.ico";
#[cfg(not(target_os = "windows"))]
pub static path: &'static str = "icons/256x256.png";

pub static width: u32 = 256;
pub static height: u32 = 256;

pub fn tao() -> AnyResult<tao::window::Icon> {
    use tao::dpi::PhysicalSize;
    use tao::platform::windows::IconExtWindows;
    use tao::window::Icon;

    let size = PhysicalSize::new(width, height);
    let icon = Icon::from_path(path, Some(size))?;
    Ok(icon)
}
