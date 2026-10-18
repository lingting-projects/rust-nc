use std::sync::LazyLock;

pub use lib_core::url::fast;

/// 关键字: 直连
pub const KEY_DIRECT: &str = "direct";
/// 关键字: 代理
pub const KEY_PROXY: &str = "proxy";
/// 关键字: 拒绝
pub const KEY_REJECT: &str = "reject";

/// 直连出口名称
pub const OUT_DIRECT: &str = "直连";
/// 下载远程规则集使用的 http 客户端名称
pub const TAG_HTTP_CLIENT: &str = "http-client";
/// 节点选择组
pub const TAG_SELECTOR: &str = "节点选择";
/// 自动选择组
pub const TAG_AUTO: &str = "自动选择";
/// 默认选择组
pub const TAG_FALLBACK: &str = "默认选择";

/// 连通性测试地址
pub const TEST_URL: &str = "http://www.gstatic.com/generate_204";

/// geoip 数据地址
pub const URL_GEOIP: &str =
    "https://github.com/MetaCubeX/meta-rules-dat/releases/download/latest/geoip-lite.dat";
/// geosite 数据地址
pub const URL_GEOSITE: &str =
    "https://github.com/MetaCubeX/meta-rules-dat/releases/download/latest/geosite.dat";
/// mmdb 数据地址
pub const URL_MMDB: &str =
    "https://github.com/MetaCubeX/meta-rules-dat/releases/download/latest/country-lite.mmdb";
/// sing-box geoip 规则集地址
pub const GEO_IP_CN: &str =
    "https://raw.githubusercontent.com/SagerNet/sing-geoip/rule-set/geoip-cn.srs";

/// clash 出口名称
pub const CLASH_TAG_DIRECT: &str = "DIRECT";
pub const CLASH_TAG_REJECT: &str = "REJECT";

/// 环回地址
pub const LOOPBACK_IPV4: &str = "127.0.0.2";
pub const LOOPBACK_IPV6: &str = "::2";
/// 路由地址
pub const ROUTE_IPV4: &str = "0.0.0.0/1";
pub const ROUTE_IPV6: &str = "::/1";
/// fake ip 网段, 172.20.0.0 - 172.23.0.0
pub const FAKE_IPV4: &str = "172.20.0.0/14";
/// 约 1844 亿亿个 ip
pub const FAKE_IPV6: &str = "fd93:0d0b:4e8a:2233::/64";
/// 虚拟网段
pub const VIRTUAL_IPV4: &str = "172.16.0.0/24";
pub const VIRTUAL_IPV6: &str = "fd93:0d0b:4e8a:0::/64";

/// 内网网段
pub static INNER_IPV4: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec![
        "192.168.0.0/16".to_string(),
        "172.16.0.0/12".to_string(),
        "10.0.0.0/8".to_string(),
    ]
});
pub static INNER_IPV6: LazyLock<Vec<String>> =
    LazyLock::new(|| vec!["fd00::/7".to_string(), "fc00::/7".to_string()]);

/// fake ip 过滤列表
pub static FAKE_IP_FILTER: LazyLock<Vec<String>> = LazyLock::new(|| {
    [
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
    ]
    .iter()
    .map(|item| item.to_string())
    .collect()
});

/// sing-box dns 服务标签
pub const TAG_DNS_CN: &str = "dns-cn";
pub const TAG_DNS_FAKE: &str = "dns-fake";
pub const TAG_DNS_PROXY: &str = "dns-proxy";
