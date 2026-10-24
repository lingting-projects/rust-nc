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
use lib_pick::value::json_to_yml;
use serde::Serialize;
use serde_yaml::Value as Yml;

use crate::source::Source;

use anytls::Anytls;
use hysteria2::Hysteria2;
use shadowsocks::ShadowSocks;
use trojan::Trojan;
use vless::Vless;
use vmess::Vmess;

/// 各协议共用的 clash 节点键: 与协议无关的连接选项。
pub(super) const COMMON_KEYS: &[&str] = &[
    "udp",
    "tfo",
    "mptcp",
    "ip-version",
    "interface-name",
    "routing-mark",
    "dialer-proxy",
    "smux",
];

/// 已由代理字段输出的节点键。
const OUTPUT_KEYS: &[&str] = &["type", "scheme", "name", "server", "port", "password"];

/// clash 代理。
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Proxy {
    pub name: String,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(rename = "skip-cert-verify")]
    pub skip_cert_verify: bool,
    pub server: String,
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(flatten)]
    pub attributes: IndexMap<String, Yml>,
}

impl Proxy {
    /// 由节点构建代理, 按协议分派给该协议自己的转换类型。
    pub(crate) fn node(node: &PickNode, source: Source) -> Self {
        match node.protocol.as_str() {
            Vless::PROTOCOL => Vless::node(node, source),
            Vmess::PROTOCOL => Vmess::node(node, source),
            Trojan::PROTOCOL => Trojan::node(node, source),
            ShadowSocks::PROTOCOL | ShadowSocks::ALIAS => ShadowSocks::node(node, source),
            Hysteria2::PROTOCOL | Hysteria2::ALIAS => Hysteria2::node(node, source),
            Anytls::PROTOCOL => Anytls::node(node, source),
            _ => Self::unknown(node),
        }
    }

    /// 协议类型填充私有字段后组装代理。
    pub(super) fn build(node: &PickNode, type_: &str, attributes: IndexMap<String, Yml>) -> Self {
        Self {
            name: node.name(),
            type_: type_.to_string(),
            skip_cert_verify: node.skip_cert_verify(),
            server: node.server(),
            port: node.port().unwrap_or(0),
            password: node.string("password"),
            attributes,
        }
    }

    /// 未识别协议的代理。
    ///
    /// 无法判断哪些字段属于该协议, 因此原样沿用节点内容, 避免丢失节点参数。
    fn unknown(node: &PickNode) -> Self {
        let attributes: IndexMap<String, Yml> = node
            .map
            .iter()
            .filter(|(key, _)| !OUTPUT_KEYS.contains(&key.as_str()))
            .map(|(key, value)| (key.clone(), json_to_yml(value)))
            .collect();

        Self::build(node, &node.protocol, attributes)
    }
}

/// 取 clash 原生节点字段。
///
/// 协议声明的键排在前面, 共用连接选项紧随其后; 两者都不包含的键不写入。
pub(super) fn clash_keys(node: &PickNode, keys: &[&str]) -> IndexMap<String, Yml> {
    let mut attributes = IndexMap::new();
    for key in keys.iter().chain(COMMON_KEYS) {
        if let Some(value) = node.map.get(*key) {
            attributes.insert((*key).to_string(), json_to_yml(value));
        }
    }
    attributes
}

/// 取分享链接字段, 右侧为 clash 字段名。
pub(super) fn link_keys(node: &PickNode, keys: &[(&str, &str)]) -> IndexMap<String, Yml> {
    let mut attributes = IndexMap::new();
    for (from, to) in keys {
        if let Some(value) = node.map.get(*from) {
            attributes.insert((*to).to_string(), json_to_yml(value));
        }
    }
    attributes
}
