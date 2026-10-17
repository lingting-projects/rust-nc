use std::path::Path;

use anyhow::{Result, bail};

/// 当前平台暂不支持打开目录。
pub fn open_path(_: &Path) -> Result<()> {
    bail!("当前平台暂不支持打开目录")
}
