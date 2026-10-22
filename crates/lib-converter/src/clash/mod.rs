mod dns;
mod group;
mod proxy;
mod rule;

pub use rule::ClashRule;

use anyhow::Result;
use indexmap::IndexMap;
use serde::Serialize;

use crate::constants::{TAG_FALLBACK, URL_GEOIP, URL_GEOSITE, URL_MMDB};
use crate::platform::Kernel;
use crate::source::Source;

use dns::{DnsConfig, build_dns};
use group::{ProxyGroup, build_proxy_groups};
use proxy::Proxy;
use rule::build_rules;

/// 生成 clash 配置。
pub(crate) fn build(kernel: &Kernel, source: Source) -> Result<String> {
    let (rule_providers, rules, rule_names) = build_rules(kernel);
    let dns = build_dns(kernel, rule_names);
    let proxies: Vec<Proxy> = kernel
        .nodes
        .iter()
        .map(|node| Proxy::node(node, source))
        .collect();
    let proxy_groups = build_proxy_groups(kernel);

    let config = ClashConfig {
        port: kernel.config.mixed_port,
        allow_lan: true,
        bind_address: kernel.config.mixed_listen.clone(),
        mode: "rule".to_string(),
        log_level: kernel.config.log_level.as_str().to_string(),
        // clash 内核的面板地址是单个 host:port 字符串
        external_controller: format!("{}:{}", kernel.config.ui_listen, kernel.config.ui_port),
        unified_delay: true,
        tcp_concurrent: false,
        global_client_fingerprint: "chrome".to_string(),
        profile: ProfileConfig {
            store_selected: true,
            store_fake_ip: kernel.config.fake_ip,
        },
        geodata_mode: true,
        geodata_loader: "standard".to_string(),
        geo_auto_update: true,
        geo_update_interval: 24,
        geox_url: GeoxUrl {
            geoip: URL_GEOIP.to_string(),
            geosite: URL_GEOSITE.to_string(),
            mmdb: URL_MMDB.to_string(),
        },
        ipv6: kernel.config.ipv6,
        tun: TunConfig {
            enable: kernel.config.tun,
            stack: "system".to_string(),
            dns_hijack: vec!["any:53".to_string(), "tcp://any:53".to_string()],
            auto_route: true,
            auto_detect_interface: true,
            ipv6: kernel.config.ipv6,
        },
        dns,
        proxies,
        proxy_groups,
        rule_providers,
        rules,
    };

    Ok(serde_yaml::to_string(&config)?)
}

#[derive(Serialize)]
struct ClashConfig {
    port: u16,
    #[serde(rename = "allow-lan")]
    allow_lan: bool,
    #[serde(rename = "bind-address")]
    bind_address: String,
    mode: String,
    #[serde(rename = "log-level")]
    log_level: String,
    #[serde(rename = "external-controller")]
    external_controller: String,
    #[serde(rename = "unified-delay")]
    unified_delay: bool,
    #[serde(rename = "tcp-concurrent")]
    tcp_concurrent: bool,
    #[serde(rename = "global-client-fingerprint")]
    global_client_fingerprint: String,
    profile: ProfileConfig,
    #[serde(rename = "geodata-mode")]
    geodata_mode: bool,
    #[serde(rename = "geodata-loader")]
    geodata_loader: String,
    #[serde(rename = "geo-auto-update")]
    geo_auto_update: bool,
    #[serde(rename = "geo-update-interval")]
    geo_update_interval: u64,
    #[serde(rename = "geox-url")]
    geox_url: GeoxUrl,
    ipv6: bool,
    tun: TunConfig,
    dns: DnsConfig,
    proxies: Vec<Proxy>,
    #[serde(rename = "proxy-groups")]
    proxy_groups: Vec<ProxyGroup>,
    #[serde(rename = "rule-providers")]
    rule_providers: IndexMap<String, ClashRule>,
    rules: Vec<String>,
}

#[derive(Serialize)]
struct ProfileConfig {
    #[serde(rename = "store-selected")]
    store_selected: bool,
    #[serde(rename = "store-fake-ip")]
    store_fake_ip: bool,
}

#[derive(Serialize)]
struct GeoxUrl {
    geoip: String,
    geosite: String,
    mmdb: String,
}

#[derive(Serialize)]
struct TunConfig {
    enable: bool,
    stack: String,
    #[serde(rename = "dns-hijack")]
    dns_hijack: Vec<String>,
    #[serde(rename = "auto-route")]
    auto_route: bool,
    #[serde(rename = "auto-detect-interface")]
    auto_detect_interface: bool,
    ipv6: bool,
}
