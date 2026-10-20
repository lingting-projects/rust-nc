use std::path::PathBuf;

use anyhow::Result;
use lib_core::{directorys, url};

/// 内置规则文件根目录。
///
/// 与配置目录分开, 多个配置共用同一份规则文件。
pub fn inner_rule_root() -> Result<PathBuf> {
    Ok(directorys()?.inner_rule.clone())
}

/// 内置规则文件: `{inner_rule}/{id}.srs`。
pub fn inner_rule_file(id: &str) -> Result<PathBuf> {
    Ok(inner_rule_root()?.join(format!("{id}.srs")))
}

/// 下载地址。
///
/// 数据库只保存原始地址, 加速镜像在下载时按需拼接, 便于随时更换镜像。
pub fn inner_rule_url(origin: &str) -> String {
    url::fast(origin)
}
