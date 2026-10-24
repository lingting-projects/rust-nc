use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// 订阅源目录, 相对 crate 根目录。
pub const SOURCE_DIR: &str = "assets/source";
/// 用于保留源目录的占位文件名, 不作为源。
pub const KEEP_FILE: &str = ".gitkeep";

/// 单个订阅源。
#[derive(Debug)]
pub struct Source {
    /// 文件名, 用于标识用例
    pub name: String,
    /// 文件内容
    pub content: String,
}

/// 读取源目录下的全部订阅源, 按文件名排序。
pub fn load_all() -> Result<Vec<Source>> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(SOURCE_DIR);
    let mut paths: Vec<PathBuf> = Vec::new();
    for item in fs::read_dir(&dir).with_context(|| format!("读取源目录失败: {}", dir.display()))?
    {
        let path = item?.path();
        if !path.is_file() || is_keep(&path) {
            continue;
        }
        paths.push(path);
    }
    paths.sort();

    paths.iter().map(|path| load(path)).collect()
}

/// 读取单个源文件。
fn load(path: &Path) -> Result<Source> {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .with_context(|| format!("源文件缺少文件名: {}", path.display()))?;
    let content =
        fs::read_to_string(path).with_context(|| format!("读取源文件失败: {}", path.display()))?;

    Ok(Source { name, content })
}

/// 是否为占位文件。
fn is_keep(path: &Path) -> bool {
    path.file_name().is_some_and(|name| name == KEEP_FILE)
}
