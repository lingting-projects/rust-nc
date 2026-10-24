//! 测试相关路径。

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// 工作区根目录, 即 crate 根目录的上两级。
pub fn workspace_root() -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    dir.parent()
        .and_then(Path::parent)
        .unwrap_or(dir)
        .to_path_buf()
}

/// 当前测试可执行文件所在的 profile 目录, 即 `target/{profile}`。
///
/// 构建脚本已把内核复制到该目录, 集成测试位于 `deps` 子目录下。
pub fn profile_dir() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("获取当前可执行文件失败")?;
    let dir = exe.parent().context("未找到当前可执行文件所在目录")?;
    let dir = match dir.file_name().is_some_and(|name| name == "deps") {
        true => dir.parent().context("未找到 target profile 目录")?,
        false => dir,
    };

    Ok(dir.to_path_buf())
}
