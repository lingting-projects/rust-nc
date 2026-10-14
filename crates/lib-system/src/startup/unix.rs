//! 非 windows 平台暂未实现开机自启。

use std::path::Path;

use anyhow::{Result, bail};

pub fn is_startup() -> Result<bool> {
    Ok(false)
}

pub fn enable(_cache_dir: &Path) -> Result<bool> {
    bail!("当前平台暂不支持开机自启")
}

pub fn disable() -> Result<bool> {
    Ok(false)
}
