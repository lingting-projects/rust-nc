use serde::Serialize;

use crate::constants::{
    FAKE_IPV4, FAKE_IPV6, KEY_DIRECT, KEY_PROXY, OUT_DIRECT, TAG_DNS_CN, TAG_DNS_FAKE,
    TAG_DNS_PROXY, TAG_SELECTOR,
};
use crate::platform::Kernel;

use super::route::RouteConfig;

/// sing-box dns 配置。
#[derive(Serialize)]
pub(crate) struct DnsConfig {
    #[serde(rename = "final")]
    final_: String,
    disable_cache: bool,
    disable_expire: bool,
    strategy: String,
    servers: Vec<DnsServer>,
    rules: Vec<DnsRule>,
}

/// dns 服务器。
///
/// sing-box 1.14 起只接受带 `type` 的新格式，旧的 `address` 单字段写法已被移除。
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum DnsServer {
    /// 加密 dns，例如 DoH
    Https {
        tag: String,
        server: String,
        detour: String,
    },
    /// 系统 dns，用于兼容无法直接指定服务器的地址
    Udp {
        tag: String,
        server: String,
        detour: String,
    },
    /// fake ip
    Fakeip {
        tag: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        inet4_range: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        inet6_range: Option<String>,
    },
}

impl DnsServer {
    /// 由 dns 地址构建服务器，`https://` 前缀按 DoH 处理。
    fn of(tag: &str, address: String, detour: &str) -> Self {
        if let Some(scheme) = address.strip_prefix("https://") {
            // 暂时不兼容指定 path
            let server = match scheme.find('/') {
                Some(position) => &scheme[..position],
                None => scheme,
            };
            return Self::Https {
                tag: tag.to_string(),
                server: server.to_string(),
                detour: detour.to_string(),
            };
        }

        Self::Udp {
            tag: tag.to_string(),
            server: address,
            detour: detour.to_string(),
        }
    }

    fn fake(tag: &str, inet4_range: Option<String>, inet6_range: Option<String>) -> Self {
        Self::Fakeip {
            tag: tag.to_string(),
            inet4_range,
            inet6_range,
        }
    }
}

#[derive(Serialize)]
struct DnsRule {
    rule_set: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    server: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    action: Option<String>,
}

impl DnsRule {
    fn route(rule_set: String, server: &str) -> Self {
        Self {
            rule_set,
            server: Some(server.to_string()),
            action: Some("route".to_string()),
        }
    }

    fn reject(rule_set: String) -> Self {
        Self {
            rule_set,
            server: None,
            action: Some("reject".to_string()),
        }
    }
}

/// 按路由规则集生成 dns 配置。
pub(crate) fn build(kernel: &Kernel, route: &RouteConfig) -> DnsConfig {
    let mut servers = vec![
        DnsServer::of(
            TAG_DNS_CN,
            kernel.config.dns_cn.first().cloned().unwrap_or_default(),
            OUT_DIRECT,
        ),
        DnsServer::of(
            TAG_DNS_PROXY,
            kernel.config.dns_proxy.first().cloned().unwrap_or_default(),
            TAG_SELECTOR,
        ),
    ];

    if kernel.config.fake_ip {
        servers.push(DnsServer::fake(
            TAG_DNS_FAKE,
            Some(FAKE_IPV4.to_string()),
            Some(FAKE_IPV6.to_string()),
        ));
    }

    let mut rules = Vec::new();
    for rule in &route.rule_set {
        let tag = rule.tag.as_str();
        // 仅其他类规则参与 dns 分流
        if !tag.contains("_o_") {
            continue;
        }

        if tag.starts_with(KEY_PROXY) {
            if kernel.config.fake_ip {
                rules.push(DnsRule::route(tag.to_string(), TAG_DNS_FAKE));
            }
            rules.push(DnsRule::route(tag.to_string(), TAG_DNS_PROXY));
        } else if tag.starts_with(KEY_DIRECT) {
            rules.push(DnsRule::route(tag.to_string(), TAG_DNS_CN));
        } else {
            rules.push(DnsRule::reject(tag.to_string()));
        }
    }

    DnsConfig {
        final_: TAG_DNS_CN.to_string(),
        disable_cache: false,
        disable_expire: false,
        strategy: kernel.ip_strategy().to_string(),
        servers,
        rules,
    }
}
