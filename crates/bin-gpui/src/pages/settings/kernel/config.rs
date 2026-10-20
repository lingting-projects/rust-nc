//! 内核分区中只读展示的内核配置。

use lib_converter::ip;
use lib_core::constants;
use lib_singbox::VERSION;
use types_core::dto::setting::SettingKernel;

/// 内核配置分区标题。
pub const TITLE: &str = "内核配置";

/// 只读展示项。
pub struct Info {
    /// 随包分发的 sing-box 版本。
    version: &'static str,
    /// 连通性测试地址。
    test_url: &'static str,
    /// 国内分流 DNS，仅展示内部数组的第一个。
    dns_cn: String,
    /// 代理分流 DNS，仅展示内部数组的第一个。
    dns_proxy: String,
    /// 内核配置中使用的网段，取值来自转换层的常量。
    ip: Ip,
}

/// 内核配置使用的网段。
///
/// 只做展示：这些取值由转换层按配置的 tun / fake ip / ipv6 开关决定是否写入，
/// 界面上不区分开关，逐项列出生成配置时可能用到的全部网段。
struct Ip {
    fake_ipv4: &'static str,
    fake_ipv6: &'static str,
    virtual_ipv4: &'static str,
    virtual_ipv6: &'static str,
    loopback_ipv4: &'static str,
    loopback_ipv6: &'static str,
    route_ipv4: &'static str,
    route_ipv6: &'static str,
    /// 内网网段，多个取值用逗号连接。
    inner_ipv4: String,
    inner_ipv6: String,
}

impl Info {
    /// 由内核设置取出只读项。
    ///
    /// 内核只支持一个 DNS 配置，因此每个分流只展示内部数组的第一个地址。
    pub fn of(setting: &SettingKernel) -> Self {
        Self {
            version: VERSION,
            test_url: constants::TEST_URL,
            dns_cn: first(&setting.dns_cn),
            dns_proxy: first(&setting.dns_proxy),
            ip: Ip::new(),
        }
    }

    /// 各展示项的标签与取值，顺序即界面顺序。
    pub fn rows(&self) -> [(&'static str, &str); 14] {
        [
            ("版本", self.version),
            ("延迟测试", self.test_url),
            ("国内 DNS", &self.dns_cn),
            ("代理 DNS", &self.dns_proxy),
            ("Fake IPv4", self.ip.fake_ipv4),
            ("Fake IPv6", self.ip.fake_ipv6),
            ("虚拟 IPv4", self.ip.virtual_ipv4),
            ("虚拟 IPv6", self.ip.virtual_ipv6),
            ("环回 IPv4", self.ip.loopback_ipv4),
            ("环回 IPv6", self.ip.loopback_ipv6),
            ("路由 IPv4", self.ip.route_ipv4),
            ("路由 IPv6", self.ip.route_ipv6),
            ("内网 IPv4", &self.ip.inner_ipv4),
            ("内网 IPv6", &self.ip.inner_ipv6),
        ]
    }
}

impl Ip {
    /// 读取转换层实际写入内核配置的网段。
    fn new() -> Self {
        Self {
            fake_ipv4: ip::FAKE_IPV4,
            fake_ipv6: ip::FAKE_IPV6,
            virtual_ipv4: ip::VIRTUAL_IPV4,
            virtual_ipv6: ip::VIRTUAL_IPV6,
            loopback_ipv4: ip::LOOPBACK_IPV4,
            loopback_ipv6: ip::LOOPBACK_IPV6,
            route_ipv4: ip::ROUTE_IPV4,
            route_ipv6: ip::ROUTE_IPV6,
            inner_ipv4: join(&ip::INNER_IPV4),
            inner_ipv6: join(&ip::INNER_IPV6),
        }
    }
}

/// 取数组第一个元素，空数组用空串表示未配置。
fn first(items: &[String]) -> String {
    items.first().cloned().unwrap_or_default()
}

/// 内网网段有多个取值，逐个列出，空数组用空串表示未配置。
fn join(items: &[String]) -> String {
    items.join(", ")
}
