use crate::core::AnyResult;
use crate::kernel::{
    default_mixed_listen, default_mixed_port, default_ui, key_direct, key_proxy, key_reject,
    tag_auto, tag_fallback, tag_selector, test_url, KernelConfig,
};
use crate::rule::{Rule, RuleType};
use crate::singbox::geo_ip_cn;
use crate::subscribe::SubscribeNode;
use serde_yaml::{to_string, Mapping, Value};
use std::collections::HashMap;
use std::slice::Iter;
use std::sync::LazyLock;

const url_geoip: &str =
    "https://github.com/MetaCubeX/meta-rules-dat/releases/download/latest/geoip-lite.dat";
const url_geosite: &str =
    "https://github.com/MetaCubeX/meta-rules-dat/releases/download/latest/geosite.dat";
const url_mmdb: &str =
    "https://github.com/MetaCubeX/meta-rules-dat/releases/download/latest/country-lite.mmdb";

const tag_direct: &str = "DIRECT";
const tag_reject: &str = "REJECT";

const fake_ip_filter: LazyLock<Vec<Value>> = LazyLock::new(|| {
    vec![
        Value::String("+.lan".to_string()),
        Value::String("+.local".to_string()),
        Value::String("+.localhost".to_string()),
        Value::String("+.localdomain".to_string()),
        Value::String("+.msftconnecttest.com".to_string()),
        Value::String("+.msftncsi.com".to_string()),
        Value::String("+.stun.*".to_string()),
        Value::String("+.stun.*.*".to_string()),
        Value::String("+.stun.*.*.*".to_string()),
        Value::String("+.stun.*.*.*.*".to_string()),
        Value::String("localhost.*".to_string()),
        Value::String("localhost.*.*".to_string()),
        Value::String("localhost.*.*.*".to_string()),
        Value::String("localhost.*.*.*.*".to_string()),
    ]
});

impl KernelConfig {
    pub fn clash_default(&self) -> AnyResult<String> {
        self.clash(default_ui, default_mixed_listen, default_mixed_port)
    }

    pub fn clash(&self, ui: &str, mixed_listen: &str, mixed_port: u16) -> AnyResult<String> {
        let mut map: HashMap<String, Value> = HashMap::new();
        self.clash_fill_basic(&mut map, ui, mixed_listen, mixed_port);
        self.clash_fill_tun(&mut map);
        let rule_names_proxy = self.clash_fill_rules(&mut map);
        self.clash_fill_dns(&mut map, rule_names_proxy);
        self.clash_fill_proxies(&mut map);
        self.clash_fill_groups(&mut map);

        let yml = to_string(&map)?;
        Ok(yml)
    }

    fn clash_fill_basic(
        &self,
        map: &mut HashMap<String, Value>,
        ui: &str,
        mixed_listen: &str,
        mixed_port: u16,
    ) {
        map.insert("port".to_string(), Value::Number(mixed_port.into()));
        map.insert("allow-lan".to_string(), Value::Bool(true));
        map.insert(
            "bind-address".to_string(),
            Value::String(mixed_listen.to_string()),
        );
        map.insert("mode".to_string(), Value::String("rule".to_string()));
        map.insert(
            "log-level".to_string(),
            Value::String(if self.debug { "debug" } else { "info" }.to_string()),
        );
        map.insert(
            "external-controller".to_string(),
            Value::String(ui.to_string()),
        );
        map.insert("unified-delay".to_string(), Value::Bool(true));
        map.insert("tcp-concurrent".to_string(), Value::Bool(false));
        map.insert(
            "global-client-fingerprint".to_string(),
            Value::String("chrome".to_string()),
        );

        let mut profile = Mapping::new();
        profile.insert(
            Value::String("store-selected".to_string()),
            Value::Bool(true),
        );
        if self.fake_ip {
            profile.insert(
                Value::String("store-fake-ip".to_string()),
                Value::Bool(true),
            );
        }

        map.insert("profile".to_string(), Value::Mapping(profile));

        map.insert("geodata-mode".to_string(), Value::Bool(true));
        map.insert(
            "geodata-loader".to_string(),
            Value::String("standard".to_string()),
        );
        map.insert("geo-auto-update".to_string(), Value::Bool(true));
        map.insert("geo-update-interval".to_string(), Value::Number(24.into()));

        let mut geox = Mapping::new();
        geox.insert(
            Value::String("geoip".to_string()),
            Value::String(url_geoip.to_string()),
        );
        geox.insert(
            Value::String("geosite".to_string()),
            Value::String(url_geosite.to_string()),
        );
        geox.insert(
            Value::String("mmdb".to_string()),
            Value::String(url_mmdb.to_string()),
        );
        map.insert("geox-url".to_string(), Value::Mapping(geox));

        map.insert("ipv6".to_string(), Value::Bool(self.ipv6));
    }

    fn clash_fill_tun(&self, map: &mut HashMap<String, Value>) {
        let mut tun = Mapping::new();
        tun.insert(Value::String("enable".to_string()), Value::Bool(self.tun));
        tun.insert(
            Value::String("stack".to_string()),
            Value::String("system".to_string()),
        );
        tun.insert(
            Value::String("dns-hijack".to_string()),
            Value::Sequence(vec![
                Value::String("any:53".to_string()),
                Value::String("tcp://any:53".to_string()),
            ]),
        );
        tun.insert(Value::String("auto-route".to_string()), Value::Bool(true));
        tun.insert(
            Value::String("auto-detect-interface".to_string()),
            Value::Bool(true),
        );
        tun.insert(Value::String("ipv6".to_string()), Value::Bool(self.ipv6));
        map.insert("tun".to_string(), Value::Mapping(tun));
    }

    fn clash_fill_dns(&self, map: &mut HashMap<String, Value>, rule_names_proxy: Vec<String>) {
        // 添加 DNS 配置
        let mut dns = Mapping::new();
        dns.insert(Value::String("enable".to_string()), Value::Bool(true));
        dns.insert(
            Value::String("listen".to_string()),
            Value::String("0.0.0.0:1053".to_string()),
        );
        dns.insert(Value::String("ipv6".to_string()), Value::Bool(self.ipv6));
        dns.insert(Value::String("prefer-h3".to_string()), Value::Bool(false));
        dns.insert(
            Value::String("cache-algorithm".to_string()),
            Value::String("arc".to_string()),
        );
        dns.insert(
            Value::String("use-system-hosts".to_string()),
            Value::Bool(false),
        );
        dns.insert(
            Value::String("enhanced-mode".to_string()),
            Value::String(
                if self.fake_ip {
                    "fake-ip"
                } else {
                    "redir-host"
                }
                .to_string(),
            ),
        );
        dns.insert(
            Value::String("fake-ip-range".to_string()),
            Value::String("198.18.0.1/16".to_string()),
        );

        dns.insert(
            Value::String("fake-ip-filter".to_string()),
            Value::Sequence(fake_ip_filter.clone()),
        );

        let dns_cn = self
            .dns_cn
            .iter()
            .map(|s| Value::String(s.to_string()))
            .collect::<Vec<_>>();

        // 添加默认 DNS 服务器
        dns.insert(
            Value::String("default-nameserver".to_string()),
            Value::Sequence(dns_cn.clone()),
        );
        dns.insert(
            Value::String("nameserver".to_string()),
            Value::Sequence(dns_cn.clone()),
        );
        dns.insert(
            Value::String("proxy-server-nameserver".to_string()),
            Value::Sequence(dns_cn.clone()),
        );

        // 添加 dns 策略
        let mut nameserver_policy = Mapping::new();

        // 添加 rule-set:proxy 策略
        let dns_proxy = self
            .dns_proxy
            .iter()
            .map(|s| Value::String(s.to_string()))
            .collect::<Vec<_>>();

        for name in rule_names_proxy {
            nameserver_policy.insert(
                Value::String(format!("rule-set:{}", name)),
                Value::Sequence(dns_proxy.clone()),
            );
        }

        dns.insert(
            Value::String("nameserver-policy".to_string()),
            Value::Mapping(nameserver_policy),
        );

        map.insert("dns".to_string(), Value::Mapping(dns));
    }

    fn clash_fill_proxies(&self, map: &mut HashMap<String, Value>) {
        let mut vec: Vec<Value> = Vec::new();
        self.nodes.iter().for_each(|node| {
            let mut mapping = Mapping::new();

            node.attribute.iter().for_each(|(k, v)| {
                let key = Value::String(k.clone());
                let mut values = Vec::new();

                if v.is_array() {
                    v.as_array().map(|s| {
                        s.into_iter().for_each(|i| {
                            if let Some(str) = SubscribeNode::json_v_string(i) {
                                values.push(Value::String(str))
                            }
                        })
                    });
                } else if let Some(str) = SubscribeNode::json_v_string(v) {
                    values.push(Value::String(str))
                };

                if values.len() > 1 {
                    mapping.insert(key, Value::Sequence(values));
                } else {
                    for v in values {
                        mapping.insert(key, v);
                        break;
                    }
                }
            });

            mapping.remove("allowInsecure");
            if node.node_type == "ss" || node.node_type == "shadowsocks" {
                mapping.insert(
                    Value::String("type".to_string()),
                    Value::String("ss".to_string()),
                );
            } else {
                mapping.insert(
                    Value::String("type".to_string()),
                    Value::String(node.node_type.clone()),
                );
            }

            mapping.insert(
                Value::String("skip-cert-verify".to_string()),
                Value::Bool(node.disable_ssl()),
            );
            mapping.insert(
                Value::String("name".to_string()),
                Value::String(node.name.clone()),
            );
            mapping.insert(
                Value::String("server".to_string()),
                Value::String(node.server.clone()),
            );
            if let Some(port) = node.port {
                mapping.insert(
                    Value::String("port".to_string()),
                    Value::Number(port.into()),
                );
            }
            if let Some(password) = &node.password {
                mapping.insert(
                    Value::String("password".to_string()),
                    Value::String(password.into()),
                );
            }
            vec.push(Value::Mapping(mapping))
        });

        map.insert("proxies".to_string(), Value::Sequence(vec));
    }

    fn clash_fill_groups(&self, map: &mut HashMap<String, Value>) {
        // 全部节点的 国家自动切换节点
        let auto_area = self.clash_build_node_auto_area();
        // 自动选择节点
        let mut auto_outbounds = Vec::new();
        auto_area.iter().for_each(|node| {
            let tag = node.get("name").unwrap();
            auto_outbounds.push(tag.as_str().unwrap().to_string())
        });
        let auto = self.clash_build_node_auto(tag_auto, auto_outbounds);

        let selector = self.clash_build_node_selector(
            tag_selector,
            auto_area.get(0).unwrap().get("name").unwrap().clone(),
            &auto,
            &auto_area,
        );

        let fallback = self.clash_build_node_selector(
            tag_fallback,
            Value::String(tag_direct.to_string()),
            &auto,
            &auto_area,
        );
        let mut outbounds = Vec::new();
        outbounds.push(selector);
        outbounds.push(auto);
        outbounds.push(fallback);

        for node in auto_area {
            outbounds.push(node)
        }

        map.insert("proxy-groups".to_string(), Value::Sequence(outbounds));
    }

    fn clash_build_node_auto_area(&self) -> Vec<Value> {
        let map = self.node_map_area();

        let mut vec = Vec::new();

        map.iter().for_each(|(code, nodes)| {
            let area = nodes.get(0).unwrap().area.unwrap();
            let tag = format!("[{}] {}自动", code, area.name_cn);

            let mut outbounds = Vec::new();
            nodes
                .iter()
                .for_each(|node| outbounds.push(node.name.to_string()));

            let value = self.clash_build_node_auto(&tag, outbounds);
            vec.push(value)
        });

        vec
    }

    fn clash_build_node_auto(&self, tag: &str, outbounds: Vec<String>) -> Value {
        let mut node = Mapping::new();

        node.insert(
            Value::String("name".to_string()),
            Value::String(tag.to_string()),
        );
        node.insert(
            Value::String("type".to_string()),
            Value::String("url-test".to_string()),
        );
        node.insert(
            Value::String("url".to_string()),
            Value::String(test_url.to_string()),
        );
        node.insert(
            Value::String("interval".to_string()),
            Value::Number(1800i32.into()),
        );
        node.insert(
            Value::String("tolerance".to_string()),
            Value::Number(150i32.into()),
        );

        let mut proxies = Vec::new();

        for tag in outbounds {
            proxies.push(Value::String(tag));
        }

        node.insert(
            Value::String("proxies".to_string()),
            Value::Sequence(proxies),
        );
        Value::Mapping(node)
    }
    fn clash_build_node_selector(
        &self,
        tag: &str,
        default: Value,
        auto: &Value,
        auto_area: &Vec<Value>,
    ) -> Value {
        let mut node = Mapping::new();

        node.insert(
            Value::String("name".to_string()),
            Value::String(tag.to_string()),
        );
        node.insert(
            Value::String("type".to_string()),
            Value::String("select".to_string()),
        );
        node.insert(Value::String("default".to_string()), default);
        node.insert(
            Value::String("url".to_string()),
            Value::String(test_url.to_string()),
        );
        node.insert(
            Value::String("interval".to_string()),
            Value::Number(1800i32.into()),
        );
        node.insert(
            Value::String("tolerance".to_string()),
            Value::Number(150i32.into()),
        );

        let mut proxies = Vec::new();

        proxies.push(Value::String(tag_direct.to_string()));
        proxies.push(Value::String(tag_reject.to_string()));
        proxies.push(auto.get("name").unwrap().clone());

        auto_area
            .iter()
            .for_each(|node| proxies.push(node.get("name").unwrap().clone()));

        self.nodes
            .iter()
            .for_each(|node| proxies.push(Value::String(node.name.clone())));

        node.insert(
            Value::String("proxies".to_string()),
            Value::Sequence(proxies),
        );
        Value::Mapping(node)
    }

    fn clash_fill_rules(&self, map: &mut HashMap<String, Value>) -> Vec<String> {
        let mut rules_process: Vec<HashMap<String, String>> = Vec::new();
        let mut rules_other: Vec<HashMap<String, String>> = Vec::new();
        let mut rules_ip: Vec<HashMap<String, String>> = Vec::new();

        self.clash_fill_rule(
            self.rules_reject.iter(),
            key_reject,
            &mut rules_process,
            &mut rules_other,
            &mut rules_ip,
        );

        self.clash_fill_rule(
            self.rules_direct.iter(),
            key_direct,
            &mut rules_process,
            &mut rules_other,
            &mut rules_ip,
        );

        self.clash_fill_rule(
            self.rules_proxy.iter(),
            key_proxy,
            &mut rules_process,
            &mut rules_other,
            &mut rules_ip,
        );

        let mut providers = Mapping::new();
        let mut names = Vec::new();
        Self::clash_processor_rule(rules_process, &mut names, &mut providers);
        Self::clash_processor_rule(rules_other, &mut names, &mut providers);
        Self::clash_processor_rule(rules_ip, &mut names, &mut providers);

        map.insert("rule-providers".to_string(), Value::Mapping(providers));

        let mut rules = Vec::new();
        let mut rule_names_proxy = Vec::new();

        for name in names {
            let mut t = "rule-set";
            let mut n = name.as_str();

            if name.ends_with("_i_geo") {
                t = "GEOIP";
                // 后面改成从字符串中提取
                n = "CN";
            }
            let o = if name.starts_with(key_direct) {
                tag_direct
            } else if name.starts_with(key_proxy) {
                rule_names_proxy.push(name.clone());
                tag_selector
            } else {
                tag_reject
            };

            rules.push(Value::String(format!("{}, {}, {}", t, n, o)));
        }
        rules.push(Value::String(format!("MATCH, {}", tag_fallback)));

        map.insert("rules".to_string(), Value::Sequence(rules));

        rule_names_proxy
    }

    fn clash_fill_rule(
        &self,
        vec: Iter<Rule>,
        prefix: &str,
        rules_process: &mut Vec<HashMap<String, String>>,
        rules_other: &mut Vec<HashMap<String, String>>,
        rules_ip: &mut Vec<HashMap<String, String>>,
    ) {
        if self.geo_cn_direct && prefix.starts_with(key_direct) {
            let rule = Rule::from_remote(RuleType::Ip, geo_ip_cn.to_string());
            let tag = format!("{}_cn_i_geo", prefix);
            rules_ip.push(rule.clash(&tag));
        }

        vec.for_each(|rule| match rule.rule_type {
            RuleType::Ip => {
                let tag = &format!("{}_i_{}", prefix, rules_ip.len());
                let rule = rule.clash(tag);
                rules_ip.push(rule);
            }
            RuleType::Process => {
                let tag = &format!("{}_p_{}", prefix, rules_process.len());
                let rule = rule.clash(tag);
                rules_process.push(rule);
            }
            RuleType::Other => {
                let tag = &format!("{}_o_{}", prefix, rules_other.len());
                let rule = rule.clash(tag);
                rules_other.push(rule);
            }
        });
    }

    fn clash_processor_rule(
        rules: Vec<HashMap<String, String>>,
        names: &mut Vec<String>,
        providers: &mut Mapping,
    ) {
        for mut rule in rules {
            let name = rule.remove("name").unwrap();
            names.push(name.clone());
            let mut value = Mapping::new();
            for (k, v) in rule {
                value.insert(Value::String(k), Value::String(v));
            }
            providers.insert(Value::String(name), Value::Mapping(value));
        }
    }
}
