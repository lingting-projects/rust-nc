//! clash 规则提供者与规则列表。

use indexmap::IndexMap;
use serde::Serialize;

use crate::constants::{
    CLASH_TAG_DIRECT, CLASH_TAG_REJECT, KEY_DIRECT, KEY_PROXY, KEY_REJECT, TAG_FALLBACK,
    TAG_SELECTOR,
};
use crate::platform::Kernel;
use crate::rule::{Rule, RuleType};

/// clash 规则提供者。
#[derive(Debug, Clone, Serialize)]
pub struct ClashRule {
    #[serde(skip)]
    pub name: String,
    pub format: String,
    pub behavior: String,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<u32>,
}

impl Rule {
    /// 转换为 clash 规则提供者。
    pub(crate) fn clash(&self, tag: &str) -> ClashRule {
        let name = tag.to_string();
        let format = "yaml".to_string();
        let behavior = "classical".to_string();

        if self.remote {
            return ClashRule {
                name,
                format,
                behavior,
                type_: "http".to_string(),
                url: Some(self.path.clone()),
                path: format!("./rules/{}.yml", tag),
                interval: Some(86400),
            };
        }

        ClashRule {
            name,
            format,
            behavior,
            type_: "file".to_string(),
            url: None,
            path: self.path.clone(),
            interval: None,
        }
    }
}

/// 构建规则提供者、规则列表与规则名称。
pub(crate) fn build_rules(
    kernel: &Kernel,
) -> (IndexMap<String, ClashRule>, Vec<String>, Vec<String>) {
    let mut process = Vec::new();
    let mut other = Vec::new();
    let mut ip = Vec::new();

    let geo_cn = kernel.config.geo_cn;
    let reject = kernel.config.rules_reject.iter();
    let direct = kernel.config.rules_direct.iter();
    let proxy = kernel.config.rules_proxy.iter();
    collect_rules(
        reject,
        KEY_REJECT,
        geo_cn,
        &mut process,
        &mut other,
        &mut ip,
    );
    collect_rules(
        direct,
        KEY_DIRECT,
        geo_cn,
        &mut process,
        &mut other,
        &mut ip,
    );
    collect_rules(proxy, KEY_PROXY, geo_cn, &mut process, &mut other, &mut ip);

    let mut providers = IndexMap::new();
    let mut names = Vec::new();
    collect_providers(process, &mut names, &mut providers);
    collect_providers(other, &mut names, &mut providers);
    collect_providers(ip, &mut names, &mut providers);

    let mut rules = Vec::new();
    let mut rule_names = Vec::new();
    for name in names {
        let (rule_type, target, outbound) = if name.ends_with("_i_geo") {
            ("GEOIP", "CN".to_string(), outbound_by_prefix(&name))
        } else {
            ("rule-set", name.clone(), outbound_by_prefix(&name))
        };
        rule_names.push(name);
        rules.push(format!("{}, {}, {}", rule_type, target, outbound));
    }
    rules.push(format!("MATCH, {}", TAG_FALLBACK));

    (providers, rules, rule_names)
}

fn collect_rules<'a>(
    rules: impl Iterator<Item = &'a Rule>,
    prefix: &str,
    geo_cn: bool,
    process: &mut Vec<ClashRule>,
    other: &mut Vec<ClashRule>,
    ip: &mut Vec<ClashRule>,
) {
    // geo cn 直连规则的优先级高于自定义 IP 规则
    if geo_cn && prefix == KEY_DIRECT {
        let rule = Rule::from_remote(RuleType::Ip, String::new());
        let tag = format!("{}_cn_i_geo", prefix);
        ip.push(rule.clash(&tag));
    }

    for rule in rules {
        let tag = match rule.rule_type {
            RuleType::Process => format!("{}_p_{}", prefix, process.len()),
            RuleType::Ip => format!("{}_i_{}", prefix, ip.len()),
            RuleType::Other => format!("{}_o_{}", prefix, other.len()),
        };

        match rule.rule_type {
            RuleType::Process => process.push(rule.clash(&tag)),
            RuleType::Ip => ip.push(rule.clash(&tag)),
            RuleType::Other => other.push(rule.clash(&tag)),
        }
    }
}

fn collect_providers(
    rules: Vec<ClashRule>,
    names: &mut Vec<String>,
    providers: &mut IndexMap<String, ClashRule>,
) {
    for rule in rules {
        names.push(rule.name.clone());
        // geo 规则使用内置 GEOIP 数据, 无需规则提供者
        if !rule.name.ends_with("_geo") {
            providers.insert(rule.name.clone(), rule);
        }
    }
}

fn outbound_by_prefix(name: &str) -> &'static str {
    if name.starts_with(KEY_DIRECT) {
        CLASH_TAG_DIRECT
    } else if name.starts_with(KEY_PROXY) {
        TAG_SELECTOR
    } else {
        CLASH_TAG_REJECT
    }
}
