use anyhow::Result;
use lib_converter::RuleType;
use lib_converter::ruleset::{self, RuleSet};

use crate::file;

/// 规则落盘结果, 按类型统计数量。
#[derive(Debug, Default)]
pub struct RuleFiles {
    /// 可用规则总数
    pub total: u64,
    /// IP 规则数量
    pub ip: u64,
    /// 进程规则数量
    pub process: u64,
    /// 其他规则数量
    pub other: u64,
}

/// 把规则内容写入规则目录: `{rule}/{id}/{name}.json` 与 `{name}.srs`。
///
/// 产物先写规则临时目录, 全部成功后再逐个原子替换到规则目录;
/// 任一环节失败则本次同步失败, 规则目录保持上一次的可用内容。
/// srs 由内核把 json 转换而来, 转换失败只记录日志, 已写入的 json 仍然可用。
pub async fn write(id: i64, content: &str) -> Result<RuleFiles> {
    let content = content.to_string();
    // 规则集转换需要等待内核子进程, 放到阻塞线程池, 避免占住异步线程
    tokio::task::spawn_blocking(move || write_sync(id, &content)).await?
}

/// 解析规则内容并在临时目录生成产物, 再替换到规则目录。
///
/// 规则目录不再整体重建, 因此本次未产出的规则类型要显式清掉,
/// 避免配置继续引用上一次残留的规则文件。
fn write_sync(id: i64, content: &str) -> Result<RuleFiles> {
    let sets = ruleset::parse(content)?;
    // 临时目录只放本次产物, 残留文件会干扰替换
    file::remove_rule_temp(id)?;

    let mut files = RuleFiles::default();
    let mut written = Vec::new();
    for set in sets {
        files.add(&set);
        write_set(id, &set)?;
        written.push(set.rule_type.name());
    }

    remove_stale(id, &written)?;
    Ok(files)
}

/// 删除本次未产出的规则集文件。
fn remove_stale(id: i64, written: &[&str]) -> Result<()> {
    for rule_type in [RuleType::Ip, RuleType::Process, RuleType::Other] {
        let name = rule_type.name();
        if written.contains(&name) {
            continue;
        }

        for ext in ["json", "srs"] {
            file::remove_file(&file::rule_file(id, name, ext)?)?;
        }
    }

    Ok(())
}
/// 写入单个规则集: 先在临时目录生成 json 与 srs, 再替换正式文件。
fn write_set(id: i64, set: &RuleSet) -> Result<()> {
    let name = set.rule_type.name();
    let temp = file::rule_temp_dir(id)?;
    let json = file::rule_file(id, name, "json")?;
    let srs = file::rule_file(id, name, "srs")?;
    let json_temp = file::temp_path(&temp)?;
    let srs_temp = file::temp_path(&temp)?;

    std::fs::write(&json_temp, &set.json)?;
    let srs_ready = match file::json_to_srs(&json_temp, &srs_temp) {
        Ok(()) => true,
        Err(error) => {
            log::warn!(
                "[规则] [{}] 规则集转换 srs 失败, 只保留 json: {error:#}",
                id
            );
            file::remove_file(&srs_temp)?;
            false
        }
    };

    file::rename(&json_temp, &json)?;
    log::debug!("[规则] [{}] 写入规则集: {}", id, json.display());

    if srs_ready {
        file::rename(&srs_temp, &srs)?;
        log::debug!("[规则] [{}] 写入规则集: {}", id, srs.display());
        return Ok(());
    }

    // 本次没有可用的 srs 时清掉旧的, 避免配置继续引用上一次的二进制规则集
    file::remove_file(&srs)?;
    Ok(())
}

impl RuleFiles {
    /// 累计单个规则集的数量。
    fn add(&mut self, set: &RuleSet) {
        self.total += set.count;
        match set.rule_type {
            lib_converter::RuleType::Ip => self.ip += set.count,
            lib_converter::RuleType::Process => self.process += set.count,
            lib_converter::RuleType::Other => self.other += set.count,
        }
    }
}
