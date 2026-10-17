//! 规则文本到 sing-box 规则集(JSON)的转换。

use std::collections::BTreeMap;

use anyhow::{Result, anyhow};
use serde_json::{Map, Value};

use crate::rule::RuleType;

/// 单个类型的 sing-box 规则集。
#[derive(Debug, Clone)]
pub struct RuleSet {
    /// 规则类型
    pub rule_type: RuleType,
    /// 规则集 JSON 文本, 版本固定为 2
    pub json: String,
    /// 规则数量
    pub count: u64,
}

/// 解析规则文本。
///
/// 每行一条 `类型,值`, 空行与 `#` 开头的注释行跳过;
/// `ip-cidr` 开头归为 IP 规则, `process-name` 归为进程规则, 其余归为其他规则。
/// 结果只包含存在规则的分类, 顺序固定为 IP、进程、其他。
pub fn parse(content: &str) -> Result<Vec<RuleSet>> {
    let mut ip = Vec::new();
    let mut process = Vec::new();
    let mut other: BTreeMap<String, Vec<Value>> = BTreeMap::new();

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((kind, value)) = line.split_once(',') else {
            continue;
        };
        let (kind, value) = (kind.trim(), value.trim());
        if kind.is_empty() || value.is_empty() {
            continue;
        }

        let kind = kind.to_lowercase();
        if kind.starts_with("ip-cidr") {
            ip.push(Value::String(value.to_string()));
        } else if kind == "process-name" {
            process.push(Value::String(value.to_string()));
        } else {
            let (key, value) = match kind.as_str() {
                // 端口需要输出为数字
                "dst-port" => (
                    "port".to_string(),
                    Value::from(
                        value
                            .parse::<u16>()
                            .map_err(|_| anyhow!("端口解析失败: {value}"))?,
                    ),
                ),
                _ => (kind.replace('-', "_"), Value::String(value.to_string())),
            };
            other.entry(key).or_default().push(value);
        }
    }

    let mut sets = Vec::new();
    if !ip.is_empty() {
        sets.push(build(RuleType::Ip, vec![rule("ip_cidr", ip)])?);
    }
    if !process.is_empty() {
        sets.push(build(
            RuleType::Process,
            vec![rule("process_name", process)],
        )?);
    }
    if !other.is_empty() {
        let rules = other
            .into_iter()
            .map(|(key, values)| rule(&key, values))
            .collect();
        sets.push(build(RuleType::Other, rules)?);
    }

    Ok(sets)
}

/// 构建规则集, 数量取自各条规则的取值数量。
fn build(rule_type: RuleType, rules: Vec<Value>) -> Result<RuleSet> {
    let count = rules.iter().map(count_rule).sum();
    let json = serde_json::to_string(&serde_json::json!({
        "version": 2,
        "rules": rules,
    }))?;

    Ok(RuleSet {
        rule_type,
        json,
        count,
    })
}

/// 构建单条规则: `{key: values}`。
fn rule(key: &str, values: Vec<Value>) -> Value {
    let mut map = Map::new();
    map.insert(key.to_string(), Value::Array(values));
    Value::Object(map)
}

/// 统计单条规则的取值数量。
fn count_rule(rule: &Value) -> u64 {
    let Some(object) = rule.as_object() else {
        return 0;
    };

    object
        .values()
        .map(|value| value.as_array().map_or(0, |array| array.len() as u64))
        .sum()
}
