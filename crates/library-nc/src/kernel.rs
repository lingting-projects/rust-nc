use crate::core::PRIORITY_CODES;
use crate::rule::Rule;
use crate::subscribe::SubscribeNode;
use std::string::ToString;
use std::sync::LazyLock;
use worker::console_debug;

#[derive(Default)]
pub struct KernelConfig {
    pub nodes: Vec<SubscribeNode>,
    pub debug: bool,
    pub tun: bool,
    pub fake_ip: bool,
    pub ipv6: bool,
    pub geo_cn_direct: bool,
    pub rules_direct: Vec<Rule>,
    pub rules_proxy: Vec<Rule>,
    pub rules_reject: Vec<Rule>,
    pub dns_cn: Vec<String>,
    pub dns_proxy: Vec<String>,
}

impl KernelConfig {
    pub fn with_sort(mut self) -> Self {
        self.nodes.sort_by_key(|node| match node.area {
            None => "0".to_string(),
            Some(area) => {
                if PRIORITY_CODES.contains(&area.code) {
                    PRIORITY_CODES
                        .iter()
                        .position(|code| code == &area.code)
                        .unwrap_or(0)
                        .to_string()
                } else {
                    area.code.to_string()
                }
            }
        });

        self
    }

    pub fn with_include(mut self, contains: &NodeContains, and: bool) -> Self {
        self.nodes = self
            .nodes
            .into_iter()
            .filter(|node| {
                let m = contains.is_match(node, and);
                #[cfg(feature = "log")]
                if m {
                    #[cfg(feature = "binary")]
                    log::debug!("[{}] 节点在包含列表中, 留存", node.name);
                    #[cfg(feature = "wrangler")]
                    console_debug!("[{}] 节点在包含列表中, 留存", node.name);
                }
                m
            })
            .collect();
        self
    }

    pub fn with_exclude(mut self, contains: &NodeContains, and: bool) -> Self {
        self.nodes = self
            .nodes
            .into_iter()
            .filter(|node| {
                let m = contains.is_match(node, and);
                #[cfg(feature = "log")]
                if m {
                    #[cfg(feature = "binary")]
                    log::debug!("[{}] 节点在排除列表中, 移除", node.name);
                    #[cfg(feature = "wrangler")]
                    console_debug!("[{}] 节点在排除列表中, 移除", node.name);
                }
                !m
            })
            .collect();
        self
    }

    pub fn ip_strategy(&self) -> String {
        if self.ipv6 {
            "prefer_ipv6"
        } else {
            "ipv4_only"
        }
        .to_string()
    }
}

// 多个参数值并行. 必须
#[derive(Default, Clone, Hash, Eq, PartialEq, Debug)]
pub struct NodeContains {
    pub area: Vec<String>,
    pub name_contains: Vec<String>,
}

impl NodeContains {
    pub fn is_empty(&self) -> bool {
        self.area.is_empty() || self.name_contains.is_empty()
    }

    fn match_area(&self, node: &SubscribeNode) -> bool {
        if self.area.is_empty() {
            #[cfg(feature = "binary")]
            log::debug!("[{}] 区域未设置, 区域匹配成功", node.name);
            #[cfg(feature = "wrangler")]
            console_debug!("[{}] 区域未设置, 区域匹配成功", node.name);
            return true;
        }

        let option = node.area;
        if option.is_none() {
            #[cfg(feature = "binary")]
            log::debug!("[{}] 节点无区域, 区域匹配失败", node.name);
            #[cfg(feature = "wrangler")]
            console_debug!("[{}] 节点无区域, 区域匹配失败", node.name);
            return false;
        }

        let area = option.unwrap();
        let m = self.area.contains(&area.code);
        #[cfg(feature = "binary")]
        log::debug!("[{}] 节点区域: {}, 匹配结果: {}", node.name, &area.code, m);
        #[cfg(feature = "wrangler")]
        console_debug!("[{}] 节点区域: {}, 匹配结果: {}", node.name, &area.code, m);
        m
    }

    fn match_name_contains(&self, node: &SubscribeNode) -> bool {
        if self.name_contains.is_empty() {
            log::debug!("[{}] 名称未设置, 名称匹配成功", node.name);
            #[cfg(feature = "wrangler")]
            console_debug!("[{}] 名称未设置, 名称匹配成功", node.name);
            return true;
        }

        let name = &node.name;
        let option = self
            .name_contains
            .iter()
            .find(|c| name.contains(c.as_str()));
        match option {
            None => {
                #[cfg(feature = "binary")]
                log::debug!("[{}] 节点名称匹配失败", node.name);
                #[cfg(feature = "wrangler")]
                console_debug!("[{}] 节点名称匹配失败", node.name);
                false
            }
            Some(key) => {
                #[cfg(feature = "binary")]
                log::debug!("[{}] 节点名称匹配成功, 关键字: {}", node.name, key);
                #[cfg(feature = "wrangler")]
                console_debug!("[{}] 节点名称匹配成功, 关键字: {}", node.name, key);
                true
            }
        }
    }

    pub fn is_match(&self, node: &SubscribeNode, and: bool) -> bool {
        if self.is_empty() {
            return true;
        }

        let conditions: Vec<fn(&NodeContains, &SubscribeNode) -> bool> =
            vec![|s, n| s.match_area(n), |s, n| s.match_name_contains(n)];

        if and {
            conditions.iter().all(|f| f(self, node))
        } else {
            conditions.iter().any(|f| f(self, node))
        }
    }
}

// clash ui
pub const clash_ui_url: &str =
    "https://github.com/MetaCubeX/metacubexd/archive/refs/heads/gh-pages.zip";
pub const test_url: &str = "http://www.gstatic.com/generate_204";

pub const key_direct: &str = "direct";
pub const key_proxy: &str = "proxy";
pub const key_reject: &str = "reject";

pub const route_ipv4: &str = "0.0.0.0/1";
pub const route_ipv6: &str = "::/1";
// 172.20.0.0 - 172.23.0.0
pub const fake_ipv4: &str = "172.20.0.0/14";
// 2的64次方个ip(约 1844亿亿个)
pub const fake_ipv6: &str = "fd93:0d0b:4e8a:2233::/64";
pub const virtual_ipv4: &str = "172.16.0.0/24";
pub const virtual_ipv6: &str = "fd93:0d0b:4e8a:0::/64";
pub const inner_ipv4: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec![
        "192.168.0.0/16".to_string(),
        "172.16.0.0/12".to_string(),
        "10.0.0.0/8".to_string(),
    ]
});
pub const inner_ipv6: LazyLock<Vec<String>> =
    LazyLock::new(|| vec!["fd00::/7".to_string(), "fc00::/7".to_string()]);

pub const exclude_default: LazyLock<NodeContains> = LazyLock::new(|| NodeContains {
    area: vec![
        "CN".to_string(),
        "HK".to_string(),
        "MO".to_string(),
        "TW".to_string(),
    ],
    name_contains: vec![
        "IEPL".to_string(),
        "IPLC".to_string(),
        "境外".to_string(),
        "回国".to_string(),
        "专线".to_string(),
    ],
});

pub const include_main: LazyLock<NodeContains> = LazyLock::new(|| NodeContains {
    area: vec!["SG".to_string(), "US".to_string(), "JP".to_string()],
    name_contains: vec![],
});

pub const dns_default_cn: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec![
        // 阿里
        "https://223.5.5.5/dns-query".to_string(),
        "https://223.6.6.6/dns-query".to_string(),
        // 腾讯
        "https://119.29.29.29/dns-query".to_string(),
        // 百度
        "https://180.76.76.76/dns-query".to_string(),
        // 360
        "https://101.226.4.6/dns-query".to_string(),
        "https://218.30.118.6/dns-query".to_string(),
        "https://123.125.81.6/dns-query".to_string(),
        "https://140.207.198.6/dns-query".to_string(),
    ]
});

pub const dns_default_proxy: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec![
        // Cloudflare
        "https://1.1.1.1/dns-query".to_string(),
        "https://104.16.248.249/dns-query".to_string(),
        // Google
        "https://8.8.8.8/dns-query".to_string(),
        // OpenDNS
        "https://208.67.222.22/dns-query".to_string(),
    ]
});
