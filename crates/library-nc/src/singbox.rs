use crate::core::{fast, AnyResult};
use crate::kernel::{
    clash_ui_url, fake_ipv4, fake_ipv6, inner_ipv4, inner_ipv6, key_direct, key_proxy,
    key_reject, route_ipv4, route_ipv6, test_url, virtual_ipv4, virtual_ipv6, KernelConfig,
};
use crate::rule::{Rule, RuleType};
use crate::subscribe::SubscribeNode;
use serde_json::{json, to_string, Value};
use std::collections::HashMap;
use std::slice::Iter;

pub const tag_selector: &str = "节点选择";
pub const tag_fallback: &str = "默认选择";
pub const tag_dns_cn: &str = "dns-cn";
pub const tag_dns_fake: &str = "dns-fake";
pub const tag_dns_proxy: &str = "dns-proxy";

pub const geo_ip_cn: &str =
    "https://raw.githubusercontent.com/SagerNet/sing-geoip/rule-set/geoip-cn.srs";

pub const default_ui: &str = "127.0.0.1:9090";
pub const default_mixed_listen: &str = "127.0.0.1";
pub const default_mixed_port: u16 = 7890;

impl KernelConfig {
    pub fn sing_box_default(&self) -> AnyResult<String> {
        self.sing_box(default_ui, default_mixed_listen, default_mixed_port)
    }

    pub fn sing_box(&self, ui: &str, mixed_listen: &str, mixed_port: u16) -> AnyResult<String> {
        let mut map: HashMap<String, Value> = HashMap::new();

        self.fill_log(&mut map);
        self.fill_experimental(&mut map, ui);
        self.fill_inbounds(&mut map, mixed_listen, mixed_port);
        self.fill_outbounds(&mut map);
        let tags = self.fill_route(&mut map);
        let tags_direct = tags.get(0).unwrap();
        let tags_proxy = tags.get(1).unwrap();
        self.fill_dns(&mut map, tags_direct, tags_proxy);

        let json = json!(map);
        let json = to_string(&json)?;
        Ok(json)
    }

    fn fill_log(&self, map: &mut HashMap<String, Value>) {
        let mut log: HashMap<String, Value> = HashMap::new();
        log.insert(
            "level".to_string(),
            if self.debug {
                json!("debug")
            } else {
                json!("info")
            },
        );
        log.insert("timestamp".to_string(), json!(true));

        map.insert("log".to_string(), json!(log));
    }

    fn fill_experimental(&self, map: &mut HashMap<String, Value>, ui: &str) {
        let mut cache: HashMap<String, Value> = HashMap::new();
        cache.insert("enabled".to_string(), json!(true));
        cache.insert("store_rdrc".to_string(), json!(true));
        if self.fake_ip {
            cache.insert("store_fakeip".to_string(), json!(true));
        }

        let mut clash: HashMap<String, Value> = HashMap::new();
        clash.insert("external_controller".to_string(), json!(ui));
        clash.insert("external_ui".to_string(), json!("nc-sing"));
        clash.insert(
            "external_ui_download_url".to_string(),
            json!(fast(clash_ui_url)),
        );
        clash.insert("external_ui_download_detour".to_string(), json!(key_direct));
        clash.insert("default_mode".to_string(), json!("rule"));

        let mut experimental: HashMap<String, Value> = HashMap::new();
        experimental.insert("cache_file".to_string(), json!(cache));
        experimental.insert("clash_api".to_string(), json!(clash));

        map.insert("experimental".to_string(), json!(experimental));
    }

    fn fill_inbounds(&self, map: &mut HashMap<String, Value>, listen: &str, port: u16) {
        let mut inbounds = Vec::new();

        if self.tun {
            inbounds.push(self.build_tun());
        }
        inbounds.push(self.build_mixed(listen, port));
        map.insert("inbounds".to_string(), json!(inbounds));
    }

    fn build_tun(&self) -> Value {
        let mut map: HashMap<String, Value> = HashMap::new();

        map.insert("type".to_string(), json!("tun"));
        map.insert("tag".to_string(), json!("tun-in"));
        map.insert("interface_name".to_string(), json!("NcRustTunBySingBox"));
        map.insert("auto_route".to_string(), json!(true));
        map.insert("strict_route".to_string(), json!(true));
        map.insert("endpoint_independent_nat".to_string(), json!(false));
        map.insert("udp_timeout".to_string(), json!("5m"));
        map.insert("stack".to_string(), json!("system"));
        map.insert("sniff_override_destination".to_string(), json!(false));
        map.insert("domain_strategy".to_string(), json!(self.ip_strategy()));

        if self.fake_ip {
            let mut virtual_ips = vec![virtual_ipv4];
            let mut fake_ips = vec![fake_ipv4];
            if self.ipv6 {
                virtual_ips.push(virtual_ipv6);
                fake_ips.push(fake_ipv6);
            }
            map.insert("address".to_string(), json!(virtual_ips));
            map.insert("route_address".to_string(), json!(fake_ips));
        } else {
            let mut route_ips = vec![route_ipv4];
            let mut exclude_ips = inner_ipv4.clone();
            if self.ipv6 {
                route_ips.push(route_ipv6);
                inner_ipv6.iter().for_each(|ip| {
                    exclude_ips.push(ip.clone());
                });
            }

            map.insert("address".to_string(), json!(route_ips));
            map.insert("route_exclude_address".to_string(), json!(exclude_ips));
        }

        json!(map)
    }

    fn build_mixed(&self, listen: &str, port: u16) -> Value {
        let mut map: HashMap<String, Value> = HashMap::new();
        map.insert("type".to_string(), json!("mixed"));
        map.insert("tag".to_string(), json!("mixed-in"));
        map.insert("set_system_proxy".to_string(), json!(false));
        map.insert("listen".to_string(), json!(listen));
        map.insert("listen_port".to_string(), json!(port));
        map.insert("tcp_fast_open".to_string(), json!(true));
        map.insert("tcp_multi_path".to_string(), json!(true));
        map.insert("udp_fragment".to_string(), json!(false));
        json!(map)
    }

    fn fill_outbounds(&self, map: &mut HashMap<String, Value>) {
        // 全部节点的 国家自动切换节点
        let auto_area = self.build_node_auto_area();
        // 自动选择节点
        let mut auto_outbounds = Vec::new();
        auto_area.iter().for_each(|node| {
            let tag = node.get("tag").unwrap();
            auto_outbounds.push(tag.as_str().unwrap().to_string())
        });
        let auto = self.build_node_auto("自动选择", auto_outbounds);

        let selector = self.build_node_selector(
            tag_selector,
            auto_area.get(0).unwrap().get("tag").unwrap().clone(),
            &auto,
            &auto_area,
        );

        let fallback = self.build_node_selector(tag_fallback, json!(key_direct), &auto, &auto_area);

        let mut outbounds = Vec::new();
        outbounds.push(selector);
        outbounds.push(auto);
        outbounds.push(fallback);

        for node in auto_area {
            outbounds.push(node)
        }
        self.nodes
            .iter()
            .for_each(|node| outbounds.push(self.build_node(node)));

        let mut direct = HashMap::new();
        direct.insert("tag".to_string(), key_direct.to_string());
        direct.insert("type".to_string(), key_direct.to_string());
        outbounds.push(json!(direct));

        let mut dns_out = HashMap::new();
        dns_out.insert("tag".to_string(), "dns_out".to_string());
        dns_out.insert("type".to_string(), "dns".to_string());
        outbounds.push(json!(dns_out));

        let mut reject = HashMap::new();
        reject.insert("tag".to_string(), key_reject.to_string());
        reject.insert("type".to_string(), "block".to_string());
        outbounds.push(json!(reject));

        map.insert("outbounds".to_string(), json!(outbounds));
    }

    fn build_node_auto_area(&self) -> Vec<Value> {
        let mut map: HashMap<String, Vec<&SubscribeNode>> = HashMap::new();
        self.nodes.iter().for_each(|node| {
            if node.area.is_none() {
                return;
            }

            let area = node.area.unwrap();
            let code = &area.code;

            match map.get_mut(code) {
                None => {
                    map.insert(code.clone(), vec![node]);
                }
                Some(vec) => {
                    vec.push(node);
                }
            }
        });

        let mut vec = Vec::new();

        map.iter().for_each(|(code, nodes)| {
            let area = nodes.get(0).unwrap().area.unwrap();
            let tag = format!("[{}] {}自动", code, area.name_cn);

            let mut outbounds = Vec::new();
            nodes
                .iter()
                .for_each(|node| outbounds.push(node.name.to_string()));

            let value = self.build_node_auto(&tag, outbounds);
            vec.push(value)
        });

        vec
    }

    fn build_node_auto(&self, tag: &str, outbounds: Vec<String>) -> Value {
        let mut node = HashMap::new();
        node.insert("tag".to_string(), json!(tag));
        node.insert("type".to_string(), json!("urltest"));
        node.insert("interrupt_exist_connections".to_string(), json!(false));
        node.insert("url".to_string(), json!(test_url));
        node.insert("interval".to_string(), json!("30s"));
        node.insert("tolerance".to_string(), json!(150));
        node.insert("outbounds".to_string(), json!(outbounds));

        json!(node)
    }

    fn build_node_selector(
        &self,
        tag: &str,
        default: Value,
        auto: &Value,
        auto_area: &Vec<Value>,
    ) -> Value {
        let mut map = HashMap::new();
        map.insert("tag".to_string(), json!(tag));
        map.insert("type".to_string(), json!("selector"));
        map.insert("interrupt_exist_connections".to_string(), json!(false));
        map.insert("default".to_string(), default);

        let mut outbounds = Vec::new();
        outbounds.push(json!(key_direct));
        outbounds.push(json!(key_reject));
        outbounds.push(auto.get("tag").unwrap().clone());

        auto_area
            .iter()
            .for_each(|node| outbounds.push(node.get("tag").unwrap().clone()));

        self.nodes
            .iter()
            .for_each(|node| outbounds.push(json!(node.name)));

        map.insert("outbounds".to_string(), json!(outbounds));

        json!(map)
    }

    fn build_node(&self, node: &SubscribeNode) -> Value {
        let mut map = HashMap::new();

        map.insert("tag".to_string(), json!(node.name.clone()));
        map.insert("type".to_string(), json!(node.node_type.clone()));
        map.insert("server".to_string(), json!(node.server.clone()));
        map.insert("server_port".to_string(), json!(node.port.unwrap_or(443)));
        map.insert("password".to_string(), json!(node.password.clone()));

        if node.node_type == "ss" || node.node_type == "shadowsocks" {
            map.insert("type".to_string(), json!("shadowsocks"));
            map.insert(
                "method".to_string(),
                json!(node.attribute.get("cipher").cloned()),
            );
        } else if node.node_type == "trojan" {
            let mut tls_config = HashMap::new();
            tls_config.insert("enabled".to_string(), json!(true));

            let insecure = node
                .attribute
                .get("skip-cert-verify")
                .map(|v| v == "true")
                .unwrap_or(false)
                || node
                    .attribute
                    .get("allowInsecure")
                    .map(|v| v == "true")
                    .unwrap_or(false);

            tls_config.insert("insecure".to_string(), json!(insecure));

            if let Some(alpn) = node.attribute.get("alpn") {
                tls_config.insert("alpn".to_string(), alpn.clone());
            }

            map.insert("tls".to_string(), json!(tls_config));
        } else {
            map.insert("type".to_string(), json!(node.node_type.clone()));
        }

        json!(map)
    }

    fn fill_route(&self, map: &mut HashMap<String, Value>) -> Vec<Vec<String>> {
        let mut route: HashMap<String, Value> = HashMap::new();
        route.insert("final".to_string(), json!(tag_fallback));
        route.insert("auto_detect_interface".to_string(), json!(true));

        let mut rules_process: Vec<HashMap<String, String>> = Vec::new();
        let mut rules_other: Vec<HashMap<String, String>> = Vec::new();
        let mut rules_ip: Vec<HashMap<String, String>> = Vec::new();

        self.fill_rule(
            self.rules_reject.iter(),
            key_reject,
            &mut rules_process,
            &mut rules_other,
            &mut rules_ip,
        );

        let tags_direct = self.fill_rule(
            self.rules_direct.iter(),
            key_direct,
            &mut rules_process,
            &mut rules_other,
            &mut rules_ip,
        );

        let tags_proxy = self.fill_rule(
            self.rules_proxy.iter(),
            key_proxy,
            &mut rules_process,
            &mut rules_other,
            &mut rules_ip,
        );

        let mut rule_set = Vec::new();
        for rule in rules_process {
            rule_set.push(rule)
        }
        for rule in rules_other {
            rule_set.push(rule)
        }
        for rule in rules_ip {
            rule_set.push(rule)
        }

        route.insert("rule_set".to_string(), json!(rule_set));

        let mut rules = Vec::new();

        let mut sniff = HashMap::new();
        sniff.insert("action".to_string(), "sniff".to_string());
        sniff.insert("timeout".to_string(), "1s".to_string());
        rules.push(sniff);

        let mut dns = HashMap::new();
        dns.insert("protocol".to_string(), "dns".to_string());
        dns.insert("action".to_string(), "hijack-dns".to_string());
        rules.push(dns);

        let mut mixed = HashMap::new();
        mixed.insert("inbound".to_string(), "mixed-in".to_string());
        mixed.insert("action".to_string(), "resolve".to_string());
        mixed.insert("strategy".to_string(), self.ip_strategy());
        rules.push(mixed);

        rule_set.iter().for_each(|rule| {
            let mut set = HashMap::new();
            let tag = rule.get("tag").map(|v| v.clone()).unwrap_or("".to_string());

            if tag.starts_with(key_direct) {
                set.insert("outbound".to_string(), key_direct.to_string());
            } else if tag.starts_with(key_proxy) {
                set.insert("outbound".to_string(), tag_selector.to_string());
            } else {
                set.insert("action".to_string(), "reject".to_string());
            }

            set.insert("rule_set".to_string(), tag);
            rules.push(set);
        });

        route.insert("rules".to_string(), json!(rules));

        map.insert("route".to_string(), json!(route));
        vec![tags_direct, tags_proxy]
    }

    fn fill_rule(
        &self,
        vec: Iter<Rule>,
        prefix: &str,
        rules_process: &mut Vec<HashMap<String, String>>,
        rules_other: &mut Vec<HashMap<String, String>>,
        rules_ip: &mut Vec<HashMap<String, String>>,
    ) -> Vec<String> {
        let mut tags = Vec::new();

        if self.geo_cn_direct {
            let rule = Rule::from_remote(RuleType::Ip, geo_ip_cn.to_string());
            let tag = format!("{}_i_geo", key_direct);
            rules_ip.push(rule.sing_box(&tag));
            tags.push(tag)
        }

        vec.for_each(|rule| match rule.rule_type {
            RuleType::Ip => {
                let rule = rule.sing_box(&format!("{}_i_{}", prefix, rules_ip.len()));
                tags.push(rule.get("tag").unwrap().clone());
                rules_ip.push(rule);
            }
            RuleType::Process => {
                let rule = rule.sing_box(&format!("{}_p_{}", prefix, rules_process.len()));
                tags.push(rule.get("tag").unwrap().clone());
                rules_process.push(rule);
            }
            RuleType::Other => {
                let rule = rule.sing_box(&format!("{}_o_{}", prefix, rules_other.len()));
                tags.push(rule.get("tag").unwrap().clone());
                rules_other.push(rule);
            }
        });

        tags
    }

    fn fill_dns(
        &self,
        map: &mut HashMap<String, Value>,
        tags_direct: &Vec<String>,
        tags_proxy: &Vec<String>,
    ) {
        let mut dns: HashMap<String, Value> = HashMap::new();
        dns.insert("final".to_string(), json!(tag_dns_cn));
        dns.insert("disable_cache".to_string(), json!(false));
        dns.insert("disable_expire".to_string(), json!(false));
        dns.insert("independent_cache".to_string(), json!(true));

        dns.insert("strategy".to_string(), json!(self.ip_strategy()));

        let servers = self.build_dns_servers();
        dns.insert("servers".to_string(), json!(servers));

        let mut rules = Vec::new();

        for tag in tags_direct {
            // 直连 只设置 other 规则的dns
            if !tag.contains("_o_") {
                continue;
            }
            let mut rule = HashMap::new();
            rule.insert("rule_set".to_string(), tag.clone());
            rule.insert("server".to_string(), tag_dns_cn.to_string());

            rules.push(rule);
        }

        // 代理 除了 ip 规则, 都要设置dns
        let rules_proxy_tags: Vec<_> = tags_proxy
            .iter()
            .filter(|tag| !tag.contains("_i_"))
            .collect();

        if self.fake_ip {
            rules_proxy_tags.iter().for_each(|tag| {
                let mut rule = HashMap::new();
                rule.insert("rule_set".to_string(), (*tag).clone());
                rule.insert("server".to_string(), tag_dns_fake.to_string());

                rules.push(rule);
            })
        }

        for tag in rules_proxy_tags {
            let mut rule = HashMap::new();
            rule.insert("server".to_string(), tag_dns_proxy.to_string());
            rule.insert("rule_set".to_string(), tag.clone());

            rules.push(rule);
        }

        dns.insert("rules".to_string(), json!(rules));

        let mut fake = HashMap::new();
        fake.insert("enabled".to_string(), json!(self.fake_ip));
        if self.fake_ip {
            fake.insert("inet4_range".to_string(), json!(fake_ipv4));
            if self.ipv6 {
                fake.insert("inet6_range".to_string(), json!(fake_ipv6));
            }
        }
        dns.insert("fakeip".to_string(), json!(fake));

        map.insert("dns".to_string(), json!(dns));
    }

    fn build_dns_servers(&self) -> Vec<HashMap<String, String>> {
        let mut servers = Vec::new();

        let mut local = HashMap::new();
        local.insert("tag".to_string(), "dns-local".to_string());
        local.insert("address".to_string(), "local".to_string());
        local.insert("detour".to_string(), key_direct.to_string());
        local.insert("strategy".to_string(), self.ip_strategy());
        servers.push(local);

        let mut cn = HashMap::new();
        cn.insert("tag".to_string(), tag_dns_cn.to_string());
        cn.insert("address".to_string(), self.dns_cn.get(0).unwrap().clone());
        cn.insert("detour".to_string(), key_direct.to_string());
        cn.insert("strategy".to_string(), self.ip_strategy());
        servers.push(cn);

        let mut proxy = HashMap::new();
        proxy.insert("tag".to_string(), tag_dns_proxy.to_string());
        proxy.insert(
            "address".to_string(),
            self.dns_proxy.get(0).unwrap().clone(),
        );
        proxy.insert("detour".to_string(), tag_selector.to_string());
        proxy.insert("strategy".to_string(), self.ip_strategy());
        servers.push(proxy);

        if self.fake_ip {
            let mut fakeip = HashMap::new();
            fakeip.insert("tag".to_string(), tag_dns_fake.to_string());
            fakeip.insert("address".to_string(), "fakeip".to_string());
            fakeip.insert("strategy".to_string(), self.ip_strategy());
            servers.push(fakeip);
        }

        servers
    }
}
