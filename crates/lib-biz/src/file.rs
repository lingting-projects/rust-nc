use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use lib_core::{directorys, next_id};

/// 规则文件根目录。
pub fn rule_root() -> Result<PathBuf> {
    Ok(directorys()?.rule.clone())
}

/// 规则文件目录: `{rule}/{id}`。
pub fn rule_dir(id: &str) -> Result<PathBuf> {
    Ok(rule_root()?.join(id))
}

/// 规则临时目录: `{rule_temp}/{id}`。
pub fn rule_temp_dir(id: &str) -> Result<PathBuf> {
    Ok(directorys()?.rule_temp.join(id))
}

/// 规则集文件: `{rule}/{id}/{name}.{ext}`。
pub fn rule_file(id: &str, name: &str, ext: &str) -> Result<PathBuf> {
    Ok(rule_dir(id)?.join(format!("{name}.{ext}")))
}

/// 配置目录: `{config}/{id}`。
pub fn config_dir(id: &str) -> Result<PathBuf> {
    Ok(directorys()?.config.join(id))
}

/// 配置临时目录: `{config_temp}/{id}`。
pub fn config_temp_dir(id: &str) -> Result<PathBuf> {
    Ok(directorys()?.config_temp.join(id))
}

/// 配置的 sing-box 配置文件: `{config}/{id}/config.json`。
pub fn sing_box_json(id: &str) -> Result<PathBuf> {
    Ok(config_dir(id)?.join("config.json"))
}

/// 内核工作目录: `{singbox_workspace}`, 内核在此目录下启动, 全部配置共用。
pub fn sing_box_workspace() -> Result<PathBuf> {
    Ok(directorys()?.singbox_workspace.clone())
}

/// 内核自身日志文件, 位于工作目录下, 每次启动重写。
pub fn sing_box_log() -> Result<PathBuf> {
    Ok(sing_box_workspace()?.join(lib_core::constants::KERNEL_LOG_FILE))
}

/// 删除规则文件目录, 连同临时目录一并清理。
pub fn remove_rule(id: &str) -> Result<()> {
    remove_dir(&rule_dir(id)?)?;
    remove_dir(&rule_temp_dir(id)?)
}

/// 删除配置文件目录, 连同临时目录一并清理。
pub fn remove_config(id: &str) -> Result<()> {
    remove_dir(&config_dir(id)?)?;
    remove_dir(&config_temp_dir(id)?)
}

/// 写入文件: 先写 `temp_dir` 下的临时文件, 再原子替换到 `path`。
///
/// 临时目录与目标目录同属应用目录, 因此替换是同一卷内的原子操作;
/// 内核随时可能读取这些文件, 不能让它看到写了一半的内容。
pub fn write(path: &Path, temp_dir: &Path, content: impl AsRef<[u8]>) -> Result<()> {
    let temp = temp_path(temp_dir)?;
    create_parent(&temp)?;
    std::fs::write(&temp, content)
        .with_context(|| format!("写入临时文件失败: {}", temp.display()))?;
    rename(&temp, path)
}

/// 用 `from` 原子替换 `to`, 目标已存在时覆盖。
///
/// 目标是内核直接引用的文件, 只做替换不做删除重建, 路径保持不变。
/// 目标被外部程序独占打开时替换会失败, 此时清理临时文件并返回失败,
/// 由调用方保留原有标识, 下一次同步再重试。
pub fn rename(from: &Path, to: &Path) -> Result<()> {
    create_parent(to)?;
    if let Err(error) = std::fs::rename(from, to) {
        let _ = std::fs::remove_file(from);
        log::error!(
            "替换文件失败: {} -> {}, {error}",
            from.display(),
            to.display()
        );
        return Err(error).with_context(|| format!("替换文件失败: {}", to.display()));
    }
    Ok(())
}

/// `temp_dir` 下本次写入使用的临时文件路径, 临时目录不存在时自动创建。
pub fn temp_path(temp_dir: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(temp_dir)
        .with_context(|| format!("创建目录失败: {}", temp_dir.display()))?;
    Ok(temp_dir.join(format!("{}.tmp", next_id()?)))
}

/// 删除规则临时目录, 不存在时视为成功。
pub fn remove_rule_temp(id: &str) -> Result<()> {
    remove_dir(&rule_temp_dir(id)?)
}

/// 删除文件, 不存在时视为成功。
pub fn remove_file(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("删除文件失败: {}", path.display())),
    }
}

/// 将 json 规则集转换为 srs 规则集。
pub fn json_to_srs(json_path: &Path, srs_path: &Path) -> Result<()> {
    crate::kernel::SingBoxInstance::instance()
        .singbox()?
        .json_to_srs(json_path, srs_path)
}

/// 创建父目录。
fn create_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("创建目录失败: {}", parent.display()))?;
    }
    Ok(())
}

/// 删除目录, 不存在时视为成功。
fn remove_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    std::fs::remove_dir_all(path).with_context(|| format!("删除目录失败: {}", path.display()))
}
