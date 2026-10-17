use indexmap::IndexMap;
use lib_pick::{PickNode, PickNodes};

use crate::node_contains::{self, NodeContains};
use crate::rule::Rule;

/// 目标平台。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// clash 内核
    Clash,
    /// sing-box 内核
    SingBox,
}

/// 平台通用配置, 由用户信息组装而成。
#[derive(Debug, Clone)]
pub struct PlatformConfig {
    /// 面板地址
    pub ui: String,
    /// 混合代理监听地址
    pub mixed_listen: String,
    /// 混合代理端口
    pub mixed_port: u16,
    /// 是否启用 tun
    pub tun: bool,
    /// 是否启用 fake ip
    pub fake_ip: bool,
    /// 是否启用 ipv6
    pub ipv6: bool,
    /// geo cn 直连
    pub geo_cn: bool,
    /// 是否输出调试日志
    pub debug: bool,
    /// 直连规则
    pub rules_direct: Vec<Rule>,
    /// 代理规则
    pub rules_proxy: Vec<Rule>,
    /// 拒绝规则
    pub rules_reject: Vec<Rule>,
    /// 国内 dns
    pub dns_cn: Vec<String>,
    /// 代理 dns
    pub dns_proxy: Vec<String>,
    /// 节点包含条件
    pub include: NodeContains,
    /// 节点排除条件
    pub exclude: NodeContains,
}

impl Default for PlatformConfig {
    fn default() -> Self {
        Self {
            ui: lib_core::constants::KERNEL_UI.to_string(),
            mixed_listen: lib_core::constants::KERNEL_MIXED_LISTEN.to_string(),
            mixed_port: lib_core::constants::KERNEL_MIXED_PORT,
            tun: true,
            fake_ip: true,
            ipv6: true,
            geo_cn: true,
            debug: false,
            rules_direct: Vec::new(),
            rules_proxy: Vec::new(),
            rules_reject: Vec::new(),
            dns_cn: lib_core::constants::KERNEL_DNS_CN
                .iter()
                .map(|item| item.to_string())
                .collect(),
            dns_proxy: lib_core::constants::KERNEL_DNS_PROXY
                .iter()
                .map(|item| item.to_string())
                .collect(),
            include: NodeContains::default_include(),
            exclude: NodeContains::default_exclude(),
        }
    }
}

/// 已完成筛选与排序的节点集合。
pub(crate) struct Kernel {
    pub(crate) nodes: Vec<PickNode>,
    pub(crate) config: PlatformConfig,
}

impl Kernel {
    /// 按配置中的包含与排除条件筛选节点。
    pub(crate) fn new(nodes: PickNodes, config: PlatformConfig) -> Self {
        let nodes = node_contains::filter(nodes, &config.include, &config.exclude);
        Self { nodes, config }
    }

    /// 按区域聚合节点, 保持节点顺序。
    pub(crate) fn map_area(&self) -> IndexMap<String, Vec<&PickNode>> {
        let mut map: IndexMap<String, Vec<&PickNode>> = IndexMap::new();
        for node in &self.nodes {
            let Some(area) = node.area else {
                continue;
            };
            map.entry(area.iso.to_string()).or_default().push(node);
        }
        map
    }

    /// ip 策略。
    pub(crate) fn ip_strategy(&self) -> &'static str {
        if self.config.ipv6 {
            "prefer_ipv6"
        } else {
            "ipv4_only"
        }
    }
}
