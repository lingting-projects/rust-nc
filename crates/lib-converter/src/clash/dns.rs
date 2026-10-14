//! clash dns 配置。

use indexmap::IndexMap;
use serde::Serialize;

use crate::constants::{FAKE_IP_FILTER, KEY_DIRECT, KEY_PROXY};
use crate::platform::Kernel;

#[derive(Serialize)]
pub(crate) struct DnsConfig {
    enable: bool,
    listen: String,
    ipv6: bool,
    #[serde(rename = "prefer-h3")]
    prefer_h3: bool,
    #[serde(rename = "cache-algorithm")]
    cache_algorithm: String,
    #[serde(rename = "use-system-hosts")]
    use_system_hosts: bool,
    #[serde(rename = "enhanced-mode")]
    enhanced_mode: String,
    #[serde(rename = "fake-ip-range")]
    fake_ip_range: String,
    #[serde(rename = "fake-ip-filter")]
    fake_ip_filter: Vec<String>,
    #[serde(rename = "default-nameserver")]
    default_nameserver: Vec<String>,
    nameserver: Vec<String>,
    #[serde(rename = "proxy-server-nameserver")]
    proxy_server_nameserver: Vec<String>,
    #[serde(rename = "nameserver-policy")]
    nameserver_policy: IndexMap<String, Vec<String>>,
}

/// 按规则名称生成 dns 配置。
pub(crate) fn build_dns(kernel: &Kernel, rule_names: Vec<String>) -> DnsConfig {
    let dns_cn = kernel.config.dns_cn.clone();
    let dns_proxy = kernel.config.dns_proxy.clone();

    let mut nameserver_policy = IndexMap::new();
    for name in rule_names {
        // IP 规则与 geo 规则不参与 dns 分流
        if name.contains("_i_") || name.ends_with("_geo") {
            continue;
        }

        if name.starts_with(KEY_DIRECT) {
            nameserver_policy.insert(format!("rule-set:{}", name), dns_cn.clone());
        } else if name.starts_with(KEY_PROXY) {
            nameserver_policy.insert(format!("rule-set:{}", name), dns_proxy.clone());
        }
    }

    DnsConfig {
        enable: true,
        listen: "0.0.0.0:1053".to_string(),
        ipv6: kernel.config.ipv6,
        prefer_h3: false,
        cache_algorithm: "arc".to_string(),
        use_system_hosts: false,
        enhanced_mode: if kernel.config.fake_ip {
            "fake-ip"
        } else {
            "redir-host"
        }
        .to_string(),
        fake_ip_range: "198.18.0.1/16".to_string(),
        fake_ip_filter: FAKE_IP_FILTER.clone(),
        default_nameserver: dns_cn.clone(),
        nameserver: dns_cn.clone(),
        proxy_server_nameserver: dns_cn,
        nameserver_policy,
    }
}
