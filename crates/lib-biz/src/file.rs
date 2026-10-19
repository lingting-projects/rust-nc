use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use lib_core::directorys;

/// 规则文件根目录。
pub fn rule_root() -> Result<PathBuf> {
    Ok(directorys()?.rule.clone())
}

/// 规则文件目录: `{rule}/{id}`。
pub fn rule_dir(id: &str) -> Result<PathBuf> {
    Ok(rule_root()?.join(id))
}

/// 规则集文件: `{rule}/{id}/{name}.{ext}`。
pub fn rule_file(id: &str, name: &str, ext: &str) -> Result<PathBuf> {
    Ok(rule_dir(id)?.join(format!("{name}.{ext}")))
}

/// 配置目录: `{config}/{id}`。
pub fn config_dir(id: &str) -> Result<PathBuf> {
    Ok(directorys()?.config.join(id))
}

/// 配置的 sing-box 工作目录: `{config}/{id}/sing_box`。
pub fn sing_box_dir(id: &str) -> Result<PathBuf> {
    Ok(config_dir(id)?.join("sing_box"))
}

/// 配置的 sing-box 配置文件: `{config}/{id}/sing_box/config.json`。
pub fn sing_box_json(id: &str) -> Result<PathBuf> {
    Ok(sing_box_dir(id)?.join("config.json"))
}

/// 内核工作目录: `{singbox_workspace}`, 内核在此目录下启动, 全部配置共用。
pub fn sing_box_workspace() -> Result<PathBuf> {
    Ok(directorys()?.singbox_workspace.clone())
}

/// 内核自身日志文件, 位于工作目录下, 每次启动重写。
pub fn sing_box_log() -> Result<PathBuf> {
    Ok(sing_box_workspace()?.join(lib_core::constants::KERNEL_LOG_FILE))
}

/// 删除规则文件目录。
pub fn remove_rule(id: &str) -> Result<()> {
    remove_dir(&rule_dir(id)?)
}

/// 删除配置文件目录。
pub fn remove_config(id: &str) -> Result<()> {
    remove_dir(&config_dir(id)?)
}

/// 写入文件, 父目录不存在时自动创建。
pub fn write(path: &Path, content: impl AsRef<[u8]>) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("创建目录失败: {}", parent.display()))?;
    }

    std::fs::write(path, content).with_context(|| format!("写入文件失败: {}", path.display()))
}

/// 复制文件, 目标目录不存在时自动创建, 目标已存在则覆盖。
pub fn copy(source: &Path, target: &Path) -> Result<()> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("创建目录失败: {}", parent.display()))?;
    }

    std::fs::copy(source, target)
        .with_context(|| format!("复制文件失败: {} -> {}", source.display(), target.display()))?;
    Ok(())
}

/// 将 json 规则集转换为 srs 规则集。
pub fn json_to_srs(json_path: &Path, srs_path: &Path) -> Result<()> {
    crate::kernel::SingBoxInstance::instance()
        .singbox()?
        .json_to_srs(json_path, srs_path)
}

/// 删除目录, 不存在时视为成功。
fn remove_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    std::fs::remove_dir_all(path).with_context(|| format!("删除目录失败: {}", path.display()))
}
