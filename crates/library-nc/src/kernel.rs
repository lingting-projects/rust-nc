use crate::rule::Rule;
use crate::subscribe::SubscribeNode;
use std::sync::LazyLock;

pub struct KernelConfig {
    pub nodes: Vec<SubscribeNode>,
    pub tun: bool,
    pub fake_ip: bool,
    pub ipv6: bool,
    pub geo_cn_ip_direct: bool,
    pub rules_direct: Vec<Rule>,
    pub rules_proxy: Vec<Rule>,
    pub rules_reject: Vec<Rule>,
    pub dns_cn: Vec<String>,
    pub dns_proxy: Vec<String>,
}

impl KernelConfig {}

// clash ui
pub const clash_ui_url: &str =
    "https://github.com/MetaCubeX/metacubexd/archive/refs/heads/gh-pages.zip";
pub const test_url : &str = "http://www.gstatic.com/generate_204";

pub const key_direct:&str = "direct";
pub const key_proxy:&str = "proxy";
pub const key_reject:&str = "reject";

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
