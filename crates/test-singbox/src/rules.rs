use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use lib_converter::{PlatformConfig, Rule, RuleType};

use crate::pipeline::Scenario;

/// 规则集目录名, 位于用例的工作目录下。
const RULE_DIR: &str = "rules";

/// 场景对应的平台配置。
///
/// 规则集使用本地文件: 内核校验时会读取文件内容, 但不会下载远程规则集,
/// 因此测试不依赖网络。规则文件在用例工作目录下按场景隔离。
pub fn config(work: &Path, scenario: Scenario) -> Result<PlatformConfig> {
    let rules = files(&work.join(scenario.name))?;

    Ok(PlatformConfig {
        tun: scenario.tun,
        fake_ip: scenario.fake_ip,
        ipv6: scenario.ipv6,
        geo_cn: scenario.geo_cn,
        rules_direct: rules.clone(),
        rules_proxy: rules.clone(),
        rules_reject: rules,
        ..PlatformConfig::default()
    })
}

/// 写入三种类型的本地规则文件。
fn files(work: &Path) -> Result<Vec<Rule>> {
    let dir = work.join(RULE_DIR);
    fs::create_dir_all(&dir).with_context(|| format!("创建规则目录失败: {}", dir.display()))?;

    let mut rules = Vec::new();
    for rule_type in [RuleType::Process, RuleType::Other, RuleType::Ip] {
        let path = dir.join(format!("{}.json", rule_type.name()));
        fs::write(&path, source(rule_type))
            .with_context(|| format!("写入规则文件失败: {}", path.display()))?;
        let path = path
            .to_str()
            .with_context(|| format!("规则文件路径非 UTF-8: {}", path.display()))?;
        rules.push(Rule::from_local(rule_type, path.to_string()));
    }

    Ok(rules)
}

/// 规则集内容, 与规则类型对应。
fn source(rule_type: RuleType) -> &'static str {
    match rule_type {
        RuleType::Process => r#"{"version":3,"rules":[{"process_name":["chrome","firefox"]}]}"#,
        RuleType::Other => {
            r#"{"version":3,"rules":[{"domain_suffix":["example.com","example.net"]}]}"#
        }
        RuleType::Ip => r#"{"version":3,"rules":[{"ip_cidr":["192.0.2.0/24","198.51.100.0/24"]}]}"#,
    }
}
