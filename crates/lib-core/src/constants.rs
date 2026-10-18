// region 内核设置

/// 内核面板监听地址
pub const KERNEL_UI_LISTEN: &str = "127.0.0.1";
/// 内核面板监听端口
pub const KERNEL_UI_PORT: u16 = 9090;
/// 混合代理监听地址
pub const KERNEL_MIXED_LISTEN: &str = "127.0.0.1";
/// 混合代理端口
pub const KERNEL_MIXED_PORT: u16 = 7890;
/// 国内 dns
pub const KERNEL_DNS_CN: &[&str] = &[
    // 阿里
    "https://223.5.5.5/dns-query",
    "https://223.6.6.6/dns-query",
    // 腾讯
    "https://119.29.29.29/dns-query",
    // 百度
    "https://180.76.76.76/dns-query",
    // 360
    "https://101.226.4.6/dns-query",
    "https://218.30.118.6/dns-query",
    "https://123.125.81.6/dns-query",
    "https://140.207.198.6/dns-query",
];
/// 代理 dns
pub const KERNEL_DNS_PROXY: &[&str] = &[
    // Cloudflare
    "https://1.1.1.1/dns-query",
    "https://104.16.248.249/dns-query",
    // Google
    "https://8.8.8.8/dns-query",
    // OpenDNS
    "https://208.67.222.22/dns-query",
];

// endregion

// region 节点筛选

/// 包含: 主区域
pub const NODE_INCLUDE_AREA: &[&str] = &["SG", "US", "JP"];
/// 包含: 是否保留无区域节点
pub const NODE_INCLUDE_AREA_NON: bool = true;
/// 排除: 区域
pub const NODE_EXCLUDE_AREA: &[&str] = &["CN", "HK", "MO", "TW"];
/// 排除: 名称关键字
pub const NODE_EXCLUDE_NAME_CONTAINS: &[&str] = &["IEPL", "IPLC", "境外", "回国", "专线"];
/// 排除: 是否移除无区域节点
pub const NODE_EXCLUDE_AREA_NON: bool = false;
/// 排除: 是否移除无名称关键字的节点
pub const NODE_EXCLUDE_NAME_NON: bool = true;

// endregion

// region 其他

/// github 加速前缀
pub const FAST_GITHUB_PREFIX: &str = "https://fastgh.lainbo.com/";
/// 连通性测试地址
pub const TEST_URL: &str = "http://www.gstatic.com/generate_204";

// endregion
