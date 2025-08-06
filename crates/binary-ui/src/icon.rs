#[cfg(target_os = "windows")]
pub static path: &'static str = "icons/256x256.ico";
#[cfg(not(target_os = "windows"))]
pub static path: &'static str = "icons/256x256.png";

pub static width: u32 = 256;
pub static height: u32 = 256;
