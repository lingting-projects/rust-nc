mod anytls;
mod hysteria2;
mod shadowsocks;
mod tls;
mod transport;
mod trojan;
mod vless;
mod vmess;

use indexmap::IndexMap;
use lib_pick::PickNode;
use serde::Serialize;
use serde_json::Value;

use crate::constants::{KEY_DIRECT, TAG_DNS_CN, TEST_URL};
use crate::source::Source;

use anytls::Anytls;
use hysteria2::Hysteria2;
use shadowsocks::ShadowSocks;
use trojan::Trojan;
use vless::Vless;
use vmess::Vmess;

/// sing-box 出口。
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Outbound {
    pub tag: String,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interrupt_exist_connections: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tolerance: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outbounds: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "server_port")]
    pub port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<OutboundTls>,
    #[serde(flatten)]
    pub attributes: IndexMap<String, Value>,
}

impl Outbound {
    /// 由节点构建出口, 按协议分派给该协议自己的转换类型。
    pub(crate) fn node(node: &PickNode, source: Source) -> Self {
        match node.protocol.as_str() {
            Vless::PROTOCOL => Vless::node(node, source),
            Vmess::PROTOCOL => Vmess::node(node, source),
            Trojan::PROTOCOL => Trojan::node(node, source),
            ShadowSocks::PROTOCOL | ShadowSocks::ALIAS => ShadowSocks::node(node, source),
            Hysteria2::PROTOCOL | Hysteria2::ALIAS => Hysteria2::node(node, source),
            Anytls::PROTOCOL => Anytls::node(node, source),
            // 未识别的协议只输出公共字段, 不写入协议私有参数
            _ => Self::unknown(node),
        }
    }

    /// 延迟测试组。
    pub(crate) fn url_test(tag: &str, outbounds: Vec<String>) -> Self {
        Self {
            tag: tag.to_string(),
            type_: "urltest".to_string(),
            interrupt_exist_connections: Some(false),
            default: None,
            url: Some(TEST_URL.to_string()),
            interval: Some("30s".to_string()),
            tolerance: Some(150),
            outbounds: Some(outbounds),
            port: None,
            server: None,
            password: None,
            tls: None,
            attributes: IndexMap::new(),
        }
    }

    /// 选择组。
    pub(crate) fn selector(tag: &str, default: String, outbounds: Vec<String>) -> Self {
        Self {
            tag: tag.to_string(),
            type_: "selector".to_string(),
            interrupt_exist_connections: Some(false),
            default: Some(default),
            url: None,
            interval: None,
            tolerance: None,
            outbounds: Some(outbounds),
            port: None,
            server: None,
            password: None,
            tls: None,
            attributes: IndexMap::new(),
        }
    }

    /// 直连出口。
    pub(crate) fn direct(tag: &str) -> Self {
        let mut attributes = IndexMap::new();
        attributes.insert("domain_resolver".to_string(), Value::from(TAG_DNS_CN));

        Self {
            tag: tag.to_string(),
            type_: KEY_DIRECT.to_string(),
            interrupt_exist_connections: None,
            default: None,
            url: None,
            interval: None,
            tolerance: None,
            outbounds: None,
            port: None,
            server: None,
            password: None,
            tls: None,
            attributes,
        }
    }

    /// 未识别协议的出口。
    fn unknown(node: &PickNode) -> Self {
        Self::build(node, &node.protocol, Some(tls::of(node)), IndexMap::new())
    }

    /// 协议类型填充私有字段后组装出口。
    pub(super) fn build(
        node: &PickNode,
        type_: &str,
        tls: Option<OutboundTls>,
        attributes: IndexMap<String, Value>,
    ) -> Self {
        Self {
            tag: node.name(),
            type_: type_.to_string(),
            interrupt_exist_connections: None,
            default: None,
            url: None,
            interval: None,
            tolerance: None,
            outbounds: None,
            port: node.port(),
            server: Some(node.server()),
            password: node.string("password"),
            tls,
            attributes,
        }
    }
}

/// sing-box tls 配置。
#[derive(Debug, Clone, Serialize)]
pub(crate) struct OutboundTls {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insecure: Option<bool>,
    #[serde(flatten)]
    pub attributes: IndexMap<String, Value>,
}

/// 按协议声明的键表取出可沿用的节点字段。
///
/// 左侧为节点键, 右侧为 sing-box 字段, 同名时直接沿用。两种来源共用: clash 的键即
/// sing-box 字段, 分享链接的键除传输层外同样可直接沿用 (例如 vless 的 `uuid`、`flow`)。
pub(super) fn keys(node: &PickNode, keys: &[(&str, &str)]) -> IndexMap<String, Value> {
    let mut attributes = IndexMap::new();
    for (from, to) in keys {
        if let Some(value) = node.map.get(*from) {
            attributes.insert((*to).to_string(), value.clone());
        }
    }
    attributes
}

/// 分享链接中的 `type` 表示传输层, sing-box 用 transport 描述; 纯 tcp 不需要。
///
/// clash 节点的网络类型在 clash 字段中, 由各协议自行处理。
pub(super) fn pure_transport(
    node: &PickNode,
    source: Source,
    attributes: &mut IndexMap<String, Value>,
) {
    if source != Source::Pure {
        return;
    }

    if let Some(transport) = transport::of(node) {
        attributes.insert("transport".to_string(), transport);
    }
}
