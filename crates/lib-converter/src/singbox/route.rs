//! sing-box 路由规则。

use serde::Serialize;

use crate::constants::{
    GEO_IP_CN, KEY_DIRECT, KEY_PROXY, KEY_REJECT, OUT_DIRECT, TAG_FALLBACK, TAG_SELECTOR,
};
use crate::platform::Kernel;
use crate::rule::{Rule, RuleType};

use super::rule::SingBoxRule;

/// sing-box 路由规则。
#[derive(Debug, Clone, Serialize)]
pub struct RouteRule {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_set: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outbound: Option<String>,
}

impl RouteRule {
    /// 嗅探域名。
    pub fn sniff() -> Self {
        Self {
            rule_set: None,
            protocol: None,
            action: "sniff".to_string(),
            timeout: Some("1s".to_string()),
            outbound: None,
        }
    }

    /// 劫持 dns。
    pub fn dns() -> Self {
        Self {
            rule_set: None,
            protocol: Some("dns".to_string()),
            action: "hijack-dns".to_string(),
            timeout: None,
            outbound: None,
        }
    }

    /// 拒绝。
    pub fn reject(rule_set: String) -> Self {
        Self {
            rule_set: Some(rule_set),
            protocol: None,
            action: "reject".to_string(),
            timeout: None,
            outbound: None,
        }
    }

    /// 指定出口。
    pub fn out(rule_set: String, outbound: String) -> Self {
        Self {
            rule_set: Some(rule_set),
            protocol: None,
            action: "route".to_string(),
            timeout: None,
            outbound: Some(outbound),
        }
    }
}

/// sing-box 路由配置。
#[derive(Serialize)]
pub(crate) struct RouteConfig {
    #[serde(rename = "final")]
    final_: String,
    auto_detect_interface: bool,
    /// dns 分流需要读取规则标签
    pub(crate) rule_set: Vec<SingBoxRule>,
    rules: Vec<RouteRule>,
}

/// 构建路由规则与规则集。
pub(crate) fn build(kernel: &Kernel) -> RouteConfig {
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

    let mut rule_set = Vec::new();
    rule_set.extend(process);
    rule_set.extend(other);
    rule_set.extend(ip);

    let mut rules = vec![RouteRule::sniff(), RouteRule::dns()];
    for rule in &rule_set {
        if rule.tag.starts_with(KEY_REJECT) {
            rules.push(RouteRule::reject(rule.tag.clone()));
            continue;
        }

        let outbound = if rule.tag.starts_with(KEY_PROXY) {
            TAG_SELECTOR
        } else {
            OUT_DIRECT
        };
        rules.push(RouteRule::out(rule.tag.clone(), outbound.to_string()));
    }

    RouteConfig {
        final_: TAG_FALLBACK.to_string(),
        auto_detect_interface: true,
        rule_set,
        rules,
    }
}

/// 按类型分类规则, geo cn 直连规则优先放入 IP 规则。
fn collect_rules<'a>(
    rules: impl Iterator<Item = &'a Rule>,
    prefix: &str,
    geo_cn: bool,
    process: &mut Vec<SingBoxRule>,
    other: &mut Vec<SingBoxRule>,
    ip: &mut Vec<SingBoxRule>,
) {
    // geo cn 直连规则的优先级高于自定义 IP 规则
    if geo_cn && prefix == KEY_DIRECT {
        let rule = Rule::from_remote(RuleType::Ip, GEO_IP_CN.to_string());
        ip.push(rule.sing_box(&geo_cn_tag(prefix)));
    }

    for rule in rules {
        let tag = match rule.rule_type {
            RuleType::Process => format!("{}_p_{}", prefix, process.len()),
            RuleType::Ip => format!("{}_i_{}", prefix, ip.len()),
            RuleType::Other => format!("{}_o_{}", prefix, other.len()),
        };

        match rule.rule_type {
            RuleType::Process => process.push(rule.sing_box(&tag)),
            RuleType::Ip => ip.push(rule.sing_box(&tag)),
            RuleType::Other => other.push(rule.sing_box(&tag)),
        }
    }
}

/// geoip cn 规则标签。
fn geo_cn_tag(prefix: &str) -> String {
    format!("{}_cn_i_geo", prefix)
}
