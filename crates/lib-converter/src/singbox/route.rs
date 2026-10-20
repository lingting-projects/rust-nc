use serde::Serialize;

use crate::constants::{
    GEO_IP_CN, INNER_RULE_GEO_IP_CN, KEY_DIRECT, KEY_PROXY, KEY_REJECT, OUT_DIRECT, TAG_DNS_CN,
    TAG_FALLBACK, TAG_HTTP_CLIENT, TAG_SELECTOR,
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
    /// 出站域名解析使用的 dns 服务器, 1.12 起取代出站内的 `domain_resolver`
    default_domain_resolver: DomainResolver,
    /// 默认 http 客户端, 1.14 起为内部组件显式指定
    default_http_client: String,
    /// dns 分流需要读取规则标签
    pub(crate) rule_set: Vec<SingBoxRule>,
    rules: Vec<RouteRule>,
}

/// 出站域名解析设置。
#[derive(Serialize)]
pub(crate) struct DomainResolver {
    pub(crate) server: String,
}

/// 构建路由规则与规则集。
pub(crate) fn build(kernel: &Kernel) -> RouteConfig {
    let mut process = Vec::new();
    let mut other = Vec::new();
    let mut ip = Vec::new();

    // geo cn 直连规则的优先级高于自定义 IP 规则
    if let Some(rule) = geo_cn_rule(kernel) {
        ip.push(rule);
    }

    let reject = kernel.config.rules_reject.iter();
    let direct = kernel.config.rules_direct.iter();
    let proxy = kernel.config.rules_proxy.iter();
    collect_rules(reject, KEY_REJECT, &mut process, &mut other, &mut ip);
    collect_rules(direct, KEY_DIRECT, &mut process, &mut other, &mut ip);
    collect_rules(proxy, KEY_PROXY, &mut process, &mut other, &mut ip);
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
        default_domain_resolver: DomainResolver {
            server: TAG_DNS_CN.to_string(),
        },
        default_http_client: TAG_HTTP_CLIENT.to_string(),
        rule_set,
        rules,
    }
}

/// 按类型分类规则。
fn collect_rules<'a>(
    rules: impl Iterator<Item = &'a Rule>,
    prefix: &str,
    process: &mut Vec<SingBoxRule>,
    other: &mut Vec<SingBoxRule>,
    ip: &mut Vec<SingBoxRule>,
) {
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

/// geo cn 直连规则集。
///
/// 内置规则目录可用时引用其中的本地文件, 内核启动不必联网下载;
/// 目录未配置时退回远程规则集, 由内核自行下载。
fn geo_cn_rule(kernel: &Kernel) -> Option<SingBoxRule> {
    if !kernel.config.geo_cn {
        return None;
    }

    let tag = geo_cn_tag(KEY_DIRECT);
    let dir = &kernel.config.inner_rule_dir;
    if !dir.as_os_str().is_empty() {
        let path = dir.join(format!("{}.srs", INNER_RULE_GEO_IP_CN));
        return Some(
            Rule::from_local(RuleType::Ip, path.to_string_lossy().to_string()).sing_box(&tag),
        );
    }

    let rule = Rule::from_remote(RuleType::Ip, GEO_IP_CN.to_string());
    Some(rule.sing_box(&tag))
}

/// geoip cn 规则标签。
fn geo_cn_tag(prefix: &str) -> String {
    format!("{}_cn_i_geo", prefix)
}
