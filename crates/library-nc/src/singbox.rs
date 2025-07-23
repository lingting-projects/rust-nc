use crate::core::fast;
use crate::kernel::{
    clash_ui_url, default_mixed_listen, default_mixed_port, default_ui, fake_ipv4, fake_ipv6,
    inner_ipv4, inner_ipv6, key_direct, key_proxy, key_reject, route_ipv4, route_ipv6, tag_auto,
    tag_fallback, tag_selector, test_url, virtual_ipv4, virtual_ipv6, KernelConfig,
};
use crate::rule::{Rule, RuleType, SingBoxRule};
use library_core::core::AnyResult;
use serde::Serialize;

pub const tag_dns_cn: &str = "dns-cn";
pub const tag_dns_fake: &str = "dns-fake";
pub const tag_dns_proxy: &str = "dns-proxy";

pub const geo_ip_cn: &str =
    "https://raw.githubusercontent.com/SagerNet/sing-geoip/rule-set/geoip-cn.srs";

#[derive(Serialize)]
struct LogConfig {
    level: String,
    timestamp: bool,
}

#[derive(Serialize)]
struct ExperimentalConfig {
    cache_file: ExperimentalCache,
    clash_api: ExperimentalClash,
}

#[derive(Serialize)]
struct ExperimentalCache {
    enabled: bool,
    store_rdrc: bool,
    store_fakeip: bool,
}

#[derive(Serialize)]
struct ExperimentalClash {
    external_controller: String,
    external_ui: String,
    external_ui_download_url: String,
    external_ui_download_detour: String,
    default_mode: String,
}

#[derive(Serialize)]
#[serde(untagged)]
enum Inbound {
    Tun {
        #[serde(rename = "type")]
        kind: String,
        tag: String,
        interface_name: String,
        auto_route: bool,
        strict_route: bool,
        endpoint_independent_nat: bool,
        udp_timeout: String,
        stack: String,
        sniff_override_destination: bool,
        domain_strategy: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        address: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        route_address: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        route_exclude_address: Option<Vec<String>>,
    },

    Mixed {
        #[serde(rename = "type")]
        kind: String,
        tag: String,
        set_system_proxy: bool,
        listen: String,
        listen_port: u16,
        tcp_fast_open: bool,
        tcp_multi_path: bool,
        udp_fragment: bool,
    },
}

#[derive(Serialize)]
struct Outbound {
    tag: String,
    #[serde(rename = "type")]
    type_: String,
    interrupt_exist_connections: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tolerance: Option<u64>,
    outbounds: Vec<String>,
}

#[derive(Serialize)]
struct RouteConfig {
    #[serde(rename = "final")]
    final_: String,
    auto_detect_interface: bool,
    rule_set: Vec<SingBoxRule>,
    rules: Vec<RouteRule>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum RouteRule {
    Action {
        #[serde(skip_serializing_if = "Option::is_none")]
        rule_set: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        inbound: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        protocol: Option<String>,
        action: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        strategy: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        timeout: Option<String>,
    },
    Outbound {
        rule_set: String,
        outbound: String,
    },
}

#[derive(Serialize)]
struct DnsConfig {
    #[serde(rename = "final")]
    final_: String,
    disable_cache: bool,
    disable_expire: bool,
    independent_cache: bool,
    strategy: String,
    servers: Vec<DnsServer>,
    rules: Vec<DnsRule>,
    #[serde(rename = "fakeip")]
    fake_ip: DnsFakeIp,
}

#[derive(Serialize)]
struct DnsServer {
    tag: String,
    address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    detour: Option<String>,
    strategy: String,
}

#[derive(Serialize)]
struct DnsFakeIp {
    enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    inet4_range: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    inet6_range: Option<String>,
}

#[derive(Serialize)]
struct DnsRule {
    server: String,
    rule_set: String,
}

#[derive(Serialize)]
struct SingBoxConfig {
    log: LogConfig,
    experimental: ExperimentalConfig,
    inbounds: Vec<Inbound>,
    outbounds: Vec<Outbound>,
    route: RouteConfig,
    dns: DnsConfig,
}

impl KernelConfig {
    pub fn sing_box_default(&self) -> AnyResult<String> {
        self.sing_box(default_ui, default_mixed_listen, default_mixed_port)
    }

    pub fn sing_box(&self, ui: &str, mixed_listen: &str, mixed_port: u16) -> AnyResult<String> {
        let (route, dns) = self.sing_box_build_dns_route();

        let config = SingBoxConfig {
            log: self.sing_box_build_log(),
            experimental: self.sing_box_build_experimental(ui),
            inbounds: self.sing_box_build_inbounds(mixed_listen, mixed_port),
            outbounds: self.sing_box_build_outbounds(),
            route,
            dns,
        };
        Ok(serde_json::to_string(&config)?)
    }

    fn sing_box_build_log(&self) -> LogConfig {
        LogConfig {
            level: if self.debug {
                "debug".into()
            } else {
                "info".into()
            },
            timestamp: true,
        }
    }

    fn sing_box_build_experimental(&self, ui: &str) -> ExperimentalConfig {
        let cache = ExperimentalCache {
            enabled: true,
            store_rdrc: true,
            store_fakeip: self.fake_ip,
        };

        let clash = ExperimentalClash {
            external_controller: ui.into(),
            external_ui: "nc-sing".into(),
            external_ui_download_url: fast(clash_ui_url),
            external_ui_download_detour: key_direct.into(),
            default_mode: "rule".into(),
        };
        ExperimentalConfig {
            cache_file: cache,
            clash_api: clash,
        }
    }

    fn sing_box_build_inbounds(&self, listen: &str, port: u16) -> Vec<Inbound> {
        let mut inbounds = Vec::new();
        if self.tun {
            inbounds.push(self.sing_box_build_tun());
        }
        inbounds.push(self.sing_box_build_mixed(listen, port));
        inbounds
    }

    fn sing_box_build_tun(&self) -> Inbound {
        let (address, route_address, route_exclude_address) = if self.fake_ip {
            let mut v_ip = vec![virtual_ipv4.into()];
            let mut f_ip = vec![fake_ipv4.into()];
            if self.ipv6 {
                v_ip.push(virtual_ipv6.into());
                f_ip.push(fake_ipv6.into());
            }
            (Some(v_ip), Some(f_ip), None)
        } else {
            let mut r_ip = vec![route_ipv4.into()];
            let mut ex_ip = inner_ipv4.clone();
            if self.ipv6 {
                r_ip.push(route_ipv6.into());
                ex_ip.extend(inner_ipv6.iter().cloned());
            }
            (Some(r_ip), None, Some(ex_ip))
        };

        Inbound::Tun {
            kind: "tun".into(),
            tag: "tun-in".into(),
            interface_name: "NcRustTunBySingBox".into(),
            auto_route: true,
            strict_route: true,
            endpoint_independent_nat: false,
            udp_timeout: "5m".into(),
            stack: "system".into(),
            sniff_override_destination: false,
            domain_strategy: self.ip_strategy(),
            address,
            route_address,
            route_exclude_address,
        }
    }

    fn sing_box_build_mixed(&self, listen: &str, port: u16) -> Inbound {
        Inbound::Mixed {
            kind: "mixed".into(),
            tag: "mixed-in".into(),
            set_system_proxy: false,
            listen: listen.into(),
            listen_port: port,
            tcp_fast_open: true,
            tcp_multi_path: true,
            udp_fragment: false,
        }
    }

    fn sing_box_build_outbounds(&self) -> Vec<Outbound> {
        let auto_area = self.sing_box_build_outbound_auto_area();
        let auto_outbounds: Vec<String> = auto_area.iter().map(|group| group.tag.clone()).collect();
        // 自动选择
        let auto = self.sing_box_build_outbound_auto(tag_auto, auto_outbounds);

        // 构建选择器组
        let default_selector = auto_area
            .first()
            .and_then(|g| Some(g.tag.clone()))
            .unwrap_or_else(|| key_direct.into());

        // 节点选择
        let selector =
            self.sing_box_build_outbound_selector(tag_selector, default_selector, &auto_area);

        // 构建fallback组
        let fallback =
            self.sing_box_build_outbound_selector(tag_fallback, key_direct.into(), &auto_area);

        // 合并所有代理组
        let mut outbounds = vec![selector, auto, fallback];
        outbounds.extend(auto_area);
        outbounds
    }

    fn sing_box_build_outbound_auto_area(&self) -> Vec<Outbound> {
        let map = self.node_map_area();

        let mut vec = Vec::new();

        map.iter().for_each(|(code, nodes)| {
            let area = nodes.get(0).unwrap().area.unwrap();
            let tag = format!("[{}] {}自动", code, area.name_cn);

            let mut outbounds = Vec::new();
            nodes
                .iter()
                .for_each(|node| outbounds.push(node.name.to_string()));

            let value = self.sing_box_build_outbound_auto(&tag, outbounds);
            vec.push(value)
        });

        vec
    }

    fn sing_box_build_outbound_auto(&self, tag: &str, outbounds: Vec<String>) -> Outbound {
        Outbound {
            tag: tag.into(),
            type_: "urltest".into(),
            interrupt_exist_connections: false,
            default: None,
            url: Some(test_url.into()),
            interval: Some("30s".into()),
            tolerance: Some(150),
            outbounds,
        }
    }

    fn sing_box_build_outbound_selector(
        &self,
        tag: &str,
        default: String,
        auto_area: &Vec<Outbound>,
    ) -> Outbound {
        let mut outbounds = Vec::new();
        outbounds.push(key_direct.into());
        outbounds.push(key_reject.into());
        outbounds.push(tag_auto.into());

        auto_area
            .iter()
            .for_each(|outbound| outbounds.push(outbound.tag.clone()));

        self.nodes
            .iter()
            .for_each(|node| outbounds.push(node.name.clone()));

        Outbound {
            tag: tag.into(),
            type_: "selector".into(),
            interrupt_exist_connections: false,
            default: Some(default),
            url: None,
            interval: None,
            tolerance: None,
            outbounds,
        }
    }

    fn sing_box_build_dns_route(&self) -> (RouteConfig, DnsConfig) {
        let route = self.sing_box_build_route();
        let dns = self.sing_box_build_dns(&route);

        (route, dns)
    }

    fn sing_box_build_route(&self) -> RouteConfig {
        let mut rules_process = Vec::new();
        let mut rules_other = Vec::new();
        let mut rules_ip = Vec::new();

        // 分类处理规则
        self.sing_box_process_rules(
            self.rules_reject.iter(),
            key_reject,
            &mut rules_process,
            &mut rules_other,
            &mut rules_ip,
        );
        self.sing_box_process_rules(
            self.rules_direct.iter(),
            key_direct,
            &mut rules_process,
            &mut rules_other,
            &mut rules_ip,
        );
        self.sing_box_process_rules(
            self.rules_proxy.iter(),
            key_proxy,
            &mut rules_process,
            &mut rules_other,
            &mut rules_ip,
        );

        let mut rule_set = Vec::new();
        rule_set.extend(rules_process);
        rule_set.extend(rules_other);
        rule_set.extend(rules_ip);

        let mut rules = vec![
            RouteRule::Action {
                rule_set: None,
                inbound: None,
                protocol: None,
                action: "sniff".into(),
                strategy: None,
                timeout: Some("1s".into()),
            },
            RouteRule::Action {
                rule_set: None,
                inbound: None,
                protocol: Some("dns".into()),
                action: "hijack-dns".into(),
                strategy: None,
                timeout: None,
            },
            RouteRule::Action {
                rule_set: None,
                inbound: Some("mixed-in".into()),
                protocol: None,
                action: "resolve".into(),
                strategy: Some(self.ip_strategy()),
                timeout: None,
            },
        ];

        rule_set.iter().for_each(|rule| {
            let rr = if rule.tag.starts_with(key_reject) {
                RouteRule::Action {
                    rule_set: Some(rule.tag.clone()),
                    inbound: None,
                    protocol: None,
                    action: "reject".into(),
                    strategy: Some(self.ip_strategy()),
                    timeout: None,
                }
            } else {
                RouteRule::Outbound {
                    rule_set: rule.tag.clone(),
                    outbound: if rule.tag.starts_with(key_proxy) {
                        tag_selector
                    } else {
                        key_direct
                    }
                    .into(),
                }
            };

            rules.push(rr)
        });

        RouteConfig {
            final_: tag_fallback.into(),
            auto_detect_interface: true,
            rule_set,
            rules,
        }
    }

    fn sing_box_process_rules<'a>(
        &self,
        rules: impl Iterator<Item = &'a Rule>,
        prefix: &str,
        rules_process: &mut Vec<SingBoxRule>,
        rules_other: &mut Vec<SingBoxRule>,
        rules_ip: &mut Vec<SingBoxRule>,
    ) {
        // 处理CN
        if self.geo_cn_direct && prefix == key_direct {
            // IP直连规则
            let rule = Rule::from_remote(RuleType::Ip, geo_ip_cn.into());
            let tag = format!("{}_cn_i_geo", prefix);
            rules_ip.push(rule.sing_box(&tag));
        }

        // 分类处理规则
        for rule in rules {
            let tag = match rule.rule_type {
                RuleType::Process => format!("{}_p_{}", prefix, rules_process.len()),
                RuleType::Ip => format!("{}_i_{}", prefix, rules_ip.len()),
                RuleType::Other => format!("{}_o_{}", prefix, rules_other.len()),
            };

            match rule.rule_type {
                RuleType::Process => rules_process.push(rule.sing_box(&tag)),
                RuleType::Ip => rules_ip.push(rule.sing_box(&tag)),
                RuleType::Other => rules_other.push(rule.sing_box(&tag)),
            }
        }
    }

    fn sing_box_build_dns(&self, route: &RouteConfig) -> DnsConfig {
        let mut servers = vec![
            DnsServer {
                tag: "dns-local".into(),
                address: "local".into(),
                detour: Some(key_direct.into()),
                strategy: self.ip_strategy(),
            },
            DnsServer {
                tag: tag_dns_cn.into(),
                address: self.dns_cn.get(0).unwrap().clone(),
                detour: Some(key_direct.into()),
                strategy: self.ip_strategy(),
            },
            DnsServer {
                tag: tag_dns_proxy.into(),
                address: self.dns_proxy.get(0).unwrap().clone(),
                detour: Some(tag_selector.into()),
                strategy: self.ip_strategy(),
            },
        ];

        if self.fake_ip {
            servers.push(DnsServer {
                tag: tag_dns_fake.into(),
                address: "fakeip".into(),
                detour: None,
                strategy: self.ip_strategy(),
            })
        }

        let rules = route
            .rule_set
            .iter()
            .filter_map(|r| {
                let tag = r.tag.as_str();

                if tag.starts_with(key_reject) || tag.contains("_i_") {
                    None
                } else {
                    let mut vec = vec![];

                    if tag.starts_with(key_proxy) {
                        if self.fake_ip {
                            vec.push(DnsRule {
                                server: tag_dns_fake.into(),
                                rule_set: tag.into(),
                            })
                        }
                        vec.push(DnsRule {
                            server: tag_dns_proxy.into(),
                            rule_set: tag.into(),
                        })
                    } else {
                        vec.push(DnsRule {
                            server: tag_dns_cn.into(),
                            rule_set: tag.into(),
                        })
                    }

                    Some(vec)
                }
            })
            .flatten()
            .collect();

        DnsConfig {
            final_: tag_dns_cn.into(),
            disable_cache: false,
            disable_expire: false,
            independent_cache: true,
            strategy: self.ip_strategy(),
            servers,
            rules,
            fake_ip: DnsFakeIp {
                enabled: self.fake_ip,
                inet4_range: self.fake_ip.then(|| fake_ipv4.into()),
                inet6_range: (self.fake_ip && self.ipv6).then(|| fake_ipv6.into()),
            },
        }
    }
}
