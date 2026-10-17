use anyhow::Result;
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
/// srs 由内核把 json 转换而来, 转换失败只记录日志, 已写入的 json 仍然可用。
pub async fn write(id: &str, content: &str) -> Result<RuleFiles> {
    let id = id.to_string();
    let content = content.to_string();
    // 规则集转换需要等待内核子进程, 放到阻塞线程池, 避免占住异步线程
    tokio::task::spawn_blocking(move || write_sync(&id, &content)).await?
}

/// 解析规则内容并写入规则目录, 写入前清理上一次的产物。
fn write_sync(id: &str, content: &str) -> Result<RuleFiles> {
    let sets = ruleset::parse(content)?;
    // 解析成功后再清理旧文件, 避免解析失败时丢失上一次的可用产物
    file::remove_rule(id)?;

    let mut files = RuleFiles::default();
    for set in sets {
        files.add(&set);
        write_set(id, &set)?;
    }

    Ok(files)
}

/// 写入单个规则集。
fn write_set(id: &str, set: &RuleSet) -> Result<()> {
    let name = set.rule_type.name();
    let json = file::rule_file(id, name, "json")?;
    let srs = file::rule_file(id, name, "srs")?;

    file::write(&json, &set.json)?;
    log::debug!("[规则] [{}] 写入规则集: {}", id, json.display());

    if let Err(error) = file::json_to_srs(&json, &srs) {
        log::warn!("[规则] [{}] 规则集转换 srs 失败, 保留 json: {error:#}", id);
        return Ok(());
    }

    log::debug!("[规则] [{}] 写入规则集: {}", id, srs.display());
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
