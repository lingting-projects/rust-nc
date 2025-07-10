use crate::core::AnyResult;
use crate::kernel::{
    default_mixed_listen, default_mixed_port, default_ui, key_direct, key_proxy, key_reject,
    tag_auto, tag_fallback, tag_selector, test_url, KernelConfig,
};
use crate::rule::{ClashRule, Rule, RuleType};
use crate::subscribe::SubscribeNode;
use serde::Serialize;
use serde_yaml::Value;
use std::collections::HashMap;
use std::sync::LazyLock;

const URL_GEOIP: &str =
    "https://github.com/MetaCubeX/meta-rules-dat/releases/download/latest/geoip-lite.dat";
const URL_GEOSITE: &str =
    "https://github.com/MetaCubeX/meta-rules-dat/releases/download/latest/geosite.dat";
const URL_MMDB: &str =
    "https://github.com/MetaCubeX/meta-rules-dat/releases/download/latest/country-lite.mmdb";

const TAG_DIRECT: &str = "DIRECT";
const TAG_REJECT: &str = "REJECT";

const FAKE_IP_FILTER_STR: &[&str] = &[
    "+.lan",
    "+.local",
    "+.localhost",
    "+.localdomain",
    "+.msftconnecttest.com",
    "+.msftncsi.com",
    "+.stun.*",
    "+.stun.*.*",
    "+.stun.*.*.*",
    "+.stun.*.*.*.*",
    "localhost.*",
    "localhost.*.*",
    "localhost.*.*.*",
    "localhost.*.*.*.*",
];

const FAKE_IP_FILTER: LazyLock<Vec<String>> =
    LazyLock::new(|| FAKE_IP_FILTER_STR.iter().map(|s| s.to_string()).collect());

#[derive(Serialize)]
struct ClashConfig {
    #[serde(rename = "port")]
    port: u16,
    #[serde(rename = "allow-lan")]
    allow_lan: bool,
    #[serde(rename = "bind-address")]
    bind_address: String,
    #[serde(rename = "mode")]
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
    #[serde(rename = "profile")]
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
    #[serde(rename = "ipv6")]
    ipv6: bool,
    #[serde(rename = "tun")]
    tun: TunConfig,
    #[serde(rename = "dns")]
    dns: DnsConfig,
    #[serde(rename = "proxies")]
    proxies: Vec<Proxy>,
    #[serde(rename = "proxy-groups")]
    proxy_groups: Vec<ProxyGroup>,
    #[serde(rename = "rule-providers")]
    rule_providers: HashMap<String, ClashRule>,
    #[serde(rename = "rules")]
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
    #[serde(rename = "geoip")]
    geoip: String,
    #[serde(rename = "geosite")]
    geosite: String,
    #[serde(rename = "mmdb")]
    mmdb: String,
}

#[derive(Serialize)]
struct TunConfig {
    #[serde(rename = "enable")]
    enable: bool,
    #[serde(rename = "stack")]
    stack: String,
    #[serde(rename = "dns-hijack")]
    dns_hijack: Vec<String>,
    #[serde(rename = "auto-route")]
    auto_route: bool,
    #[serde(rename = "auto-detect-interface")]
    auto_detect_interface: bool,
    #[serde(rename = "ipv6")]
    ipv6: bool,
}

#[derive(Serialize)]
struct DnsConfig {
    #[serde(rename = "enable")]
    enable: bool,
    #[serde(rename = "listen")]
    listen: String,
    #[serde(rename = "ipv6")]
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
    #[serde(rename = "nameserver")]
    nameserver: Vec<String>,
    #[serde(rename = "proxy-server-nameserver")]
    proxy_server_nameserver: Vec<String>,
    #[serde(rename = "nameserver-policy")]
    nameserver_policy: HashMap<String, Vec<String>>,
}

#[derive(Serialize)]
struct Proxy {
    #[serde(rename = "name")]
    name: String,
    #[serde(rename = "type")]
    type_: String,
    #[serde(flatten)]
    attributes: HashMap<String, Value>,
    #[serde(rename = "skip-cert-verify")]
    skip_cert_verify: bool,
    server: String,
    port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<String>,
}

#[derive(Serialize)]
struct ProxyGroup {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(rename = "default", skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(rename = "url")]
    pub url: String,
    #[serde(rename = "interval")]
    pub interval: u64,
    #[serde(rename = "tolerance")]
    pub tolerance: u64,
    #[serde(rename = "proxies")]
    pub proxies: Vec<String>,
}

impl KernelConfig {
    pub fn clash_default(&self) -> AnyResult<String> {
        self.clash(default_ui, default_mixed_listen, default_mixed_port)
    }

    pub fn clash(&self, ui: &str, mixed_listen: &str, mixed_port: u16) -> AnyResult<String> {
        // 构建规则相关数据
        let (rule_providers, rules, rule_names) = self.clash_build_rules();

        // 构建DNS配置
        let dns = self.clash_build_dns(rule_names);

        // 构建代理列表
        let proxies = self.clash_build_proxies();

        // 构建代理组
        let proxy_groups = self.clash_build_proxy_groups();

        // 构建完整配置
        let config = ClashConfig {
            port: mixed_port,
            allow_lan: true,
            bind_address: mixed_listen.into(),
            mode: "rule".into(),
            log_level: if self.debug { "debug" } else { "info" }.into(),
            external_controller: ui.into(),
            unified_delay: true,
            tcp_concurrent: false,
            global_client_fingerprint: "chrome".into(),
            profile: ProfileConfig {
                store_selected: true,
                store_fake_ip: self.fake_ip,
            },
            geodata_mode: true,
            geodata_loader: "standard".into(),
            geo_auto_update: true,
            geo_update_interval: 24,
            geox_url: GeoxUrl {
                geoip: URL_GEOIP.into(),
                geosite: URL_GEOSITE.into(),
                mmdb: URL_MMDB.into(),
            },
            ipv6: self.ipv6,
            tun: TunConfig {
                enable: self.tun,
                stack: "system".into(),
                dns_hijack: vec!["any:53".into(), "tcp://any:53".into()],
                auto_route: true,
                auto_detect_interface: true,
                ipv6: self.ipv6,
            },
            dns,
            proxies,
            proxy_groups,
            rule_providers,
            rules,
        };

        let yml = serde_yaml::to_string(&config)?;
        Ok(yml)
    }

    fn clash_build_rules(&self) -> (HashMap<String, ClashRule>, Vec<String>, Vec<String>) {
        let mut rules_process = Vec::new();
        let mut rules_other = Vec::new();
        let mut rules_ip = Vec::new();

        // 分类处理规则
        self.clash_process_rules(
            self.rules_reject.iter(),
            key_reject,
            &mut rules_process,
            &mut rules_other,
            &mut rules_ip,
        );
        self.clash_process_rules(
            self.rules_direct.iter(),
            key_direct,
            &mut rules_process,
            &mut rules_other,
            &mut rules_ip,
        );
        self.clash_process_rules(
            self.rules_proxy.iter(),
            key_proxy,
            &mut rules_process,
            &mut rules_other,
            &mut rules_ip,
        );

        // 构建规则提供者和规则列表
        let mut rule_providers = HashMap::new();
        let mut names: Vec<String> = Vec::new();
        self.clash_build_rule_providers(rules_process, &mut names, &mut rule_providers);
        self.clash_build_rule_providers(rules_other, &mut names, &mut rule_providers);
        self.clash_build_rule_providers(rules_ip, &mut names, &mut rule_providers);

        let mut rules = Vec::new();
        let mut rule_names = Vec::new();

        for name in names {
            let (rule_type, target, outbound) = if name.ends_with("_i_geo") {
                ("GEOIP", "CN", self.clash_outbound_by_prefix(&name))
            } else {
                (
                    "rule-set",
                    name.as_str(),
                    self.clash_outbound_by_prefix(&name),
                )
            };
            rule_names.push(name.clone());
            rules.push(format!("{}, {}, {}", rule_type, target, outbound));
        }
        rules.push(format!("MATCH, {}", tag_fallback));

        (rule_providers, rules, rule_names)
    }

    fn clash_process_rules<'a>(
        &self,
        rules: impl Iterator<Item = &'a Rule>,
        prefix: &str,
        rules_process: &mut Vec<ClashRule>,
        rules_other: &mut Vec<ClashRule>,
        rules_ip: &mut Vec<ClashRule>,
    ) {
        // 处理CN IP直连规则
        if self.geo_cn_direct && prefix == key_direct {
            let rule = Rule::from_remote(RuleType::Ip, "".into());
            let tag = format!("{}_cn_i_geo", prefix);
            rules_ip.push(rule.clash(&tag));
        }

        // 分类处理规则
        for rule in rules {
            let tag = match rule.rule_type {
                RuleType::Process => format!("{}_p_{}", prefix, rules_process.len()),
                RuleType::Ip => format!("{}_i_{}", prefix, rules_ip.len()),
                RuleType::Other => format!("{}_o_{}", prefix, rules_other.len()),
            };

            match rule.rule_type {
                RuleType::Process => rules_process.push(rule.clash(&tag)),
                RuleType::Ip => rules_ip.push(rule.clash(&tag)),
                RuleType::Other => rules_other.push(rule.clash(&tag)),
            }
        }
    }

    fn clash_build_rule_providers(
        &self,
        rules: Vec<ClashRule>,
        names: &mut Vec<String>,
        providers: &mut HashMap<String, ClashRule>,
    ) {
        for rule in rules {
            let name = rule.name.clone();
            providers.insert(name.clone(), rule);
            names.push(name);
        }
    }

    fn clash_outbound_by_prefix(&self, name: &str) -> &'static str {
        if name.starts_with(key_direct) {
            TAG_DIRECT
        } else if name.starts_with(key_proxy) {
            tag_selector
        } else {
            TAG_REJECT
        }
    }

    fn clash_build_dns(&self, rule_names: Vec<String>) -> DnsConfig {
        // 构建DNS服务器列表（避免克隆）
        let dns_cn: Vec<String> = self.dns_cn.iter().cloned().collect();
        let dns_proxy: Vec<String> = self.dns_proxy.iter().cloned().collect();

        // 构建DNS策略
        let mut nameserver_policy = HashMap::new();
        for name in rule_names {
            if name.contains("_i_") || name.ends_with("_geo") {
                continue;
            }
            if name.starts_with(key_direct) {
                nameserver_policy.insert(format!("rule-set:{}", name), dns_cn.clone());
            } else if name.starts_with(key_proxy) {
                nameserver_policy.insert(format!("rule-set:{}", name), dns_proxy.clone());
            }
        }

        DnsConfig {
            enable: true,
            listen: "0.0.0.0:1053".into(),
            ipv6: self.ipv6,
            prefer_h3: false,
            cache_algorithm: "arc".into(),
            use_system_hosts: false,
            enhanced_mode: if self.fake_ip {
                "fake-ip"
            } else {
                "redir-host"
            }
            .into(),
            fake_ip_range: "198.18.0.1/16".into(),
            fake_ip_filter: FAKE_IP_FILTER.clone(),
            default_nameserver: dns_cn.clone(),
            nameserver: dns_cn.clone(),
            proxy_server_nameserver: dns_cn,
            nameserver_policy,
        }
    }

    fn clash_build_proxies(&self) -> Vec<Proxy> {
        self.nodes
            .iter()
            .map(|node| {
                let mut attributes = HashMap::new();
                node.attribute.iter().for_each(|(k, v)| {
                    if k == "type" || k == "allowInsecure" {
                        return;
                    }
                    let value = if v.is_array() {
                        Value::Sequence(
                            v.as_array()
                                .unwrap()
                                .iter()
                                .filter_map(|i| SubscribeNode::json_v_string(i).map(Value::String))
                                .collect(),
                        )
                    } else {
                        SubscribeNode::json_v_string(v)
                            .map(Value::String)
                            .unwrap_or(Value::Null)
                    };
                    attributes.insert(k.clone(), value);
                });

                Proxy {
                    name: node.name.clone(),
                    type_: if node.node_type == "ss" || node.node_type == "shadowsocks" {
                        "ss".into()
                    } else {
                        node.node_type.clone()
                    },
                    attributes,
                    skip_cert_verify: node.disable_ssl(),
                    server: node.server.clone(),
                    port: node.port.unwrap_or(0),
                    password: node.password.clone(),
                }
            })
            .collect()
    }

    fn clash_build_proxy_groups(&self) -> Vec<ProxyGroup> {
        let auto_area = self.clash_build_node_auto_area();
        let auto_proxies: Vec<String> = auto_area
            .iter()
            .map(|group| group.name.clone())
            .collect();

        // 构建自动选择组
        let auto = self.clash_build_node_auto(tag_auto.into(), auto_proxies);

        // 构建选择器组
        let default_selector = auto_area
            .first()
            .and_then(|g| Some(g.name.clone()))
            .unwrap_or_else(|| TAG_DIRECT.into());

        // 节点选择
        let selector = self.clash_build_selector_group(tag_selector, default_selector, &auto_area);

        // 构建fallback组
        let fallback = self.clash_build_selector_group(tag_fallback, TAG_DIRECT.into(), &auto_area);

        // 合并所有代理组
        let mut groups = vec![selector, auto, fallback];
        groups.extend(auto_area);

        groups
    }

    fn clash_build_node_auto_area(&self) -> Vec<ProxyGroup> {
        let area_map = self.node_map_area();
        area_map
            .into_iter()
            .map(|(_, nodes)| {
                let area_name = nodes
                    .first()
                    .unwrap()
                    .area
                    .as_ref()
                    .unwrap()
                    .name_cn
                    .clone();
                let group_name = format!(
                    "[{}] {}自动",
                    nodes.first().unwrap().area.unwrap().code,
                    area_name
                );
                let proxies = nodes.iter().map(|n| n.name.clone()).collect();
                self.clash_build_node_auto(group_name, proxies)
            })
            .collect()
    }

    fn clash_build_node_auto(&self, tag: String, proxies: Vec<String>) -> ProxyGroup {
        ProxyGroup {
            name: tag,
            type_: "url-test".into(),
            default: None,
            url: test_url.into(),
            interval: 1800,
            tolerance: 150,
            proxies,
        }
    }

    fn clash_build_selector_group(
        &self,
        name: &str,
        default: String,
        auto_area: &[ProxyGroup],
    ) -> ProxyGroup {
        let mut proxies = vec![TAG_DIRECT.into(), TAG_REJECT.into(), tag_auto.into()];

        // 添加地区自动组
        proxies.extend(auto_area.iter().map(|g| g.name.clone()));

        // 添加原始节点
        proxies.extend(self.nodes.iter().map(|n| n.name.clone()));

        ProxyGroup {
            name: name.into(),
            type_: "select".into(),
            default: Some(default),
            url: test_url.into(),
            interval: 1800,
            tolerance: 150,
            proxies,
        }
    }
}
