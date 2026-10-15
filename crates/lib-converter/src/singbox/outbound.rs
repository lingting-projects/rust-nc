use indexmap::IndexMap;
use lib_pick::PickNode;
use serde::Serialize;
use serde_json::Value;

use crate::constants::{KEY_DIRECT, TAG_DNS_CN, TEST_URL};
use crate::source::Source;

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
    /// 延迟测试组。
    pub fn url_test(tag: &str, outbounds: Vec<String>) -> Self {
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
    pub fn selector(tag: &str, default: String, outbounds: Vec<String>) -> Self {
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
    pub fn direct(tag: &str) -> Self {
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

    /// 由节点构建出口, `source` 决定节点键的翻译方式。
    pub fn node(node: &PickNode, source: Source) -> Self {
        match source {
            Source::Clash => Self::clash_node(node),
            Source::Pure => Self::pure_node(node),
        }
    }

    /// clash 原生字段节点。
    fn clash_node(node: &PickNode) -> Self {
        let mut attributes = IndexMap::new();
        for (key, value) in &node.map {
            if skip_key(key) {
                continue;
            }
            attributes.insert(key.clone(), value.clone());
        }

        if node.protocol == "hysteria2" {
            attributes.insert("hop_interval".to_string(), Value::from("10s"));
        }

        Self::node_outbound(node, attributes, tls_of(node))
    }

    /// 分享链接字段节点。
    fn pure_node(node: &PickNode) -> Self {
        let mut attributes = IndexMap::new();

        // 分享链接中的 type 表示传输层, sing-box 用 transport 描述
        if let Some(transport) = node.string("type") {
            attributes.insert("transport".to_string(), transport_of(node, &transport));
        }

        if node.protocol == "hysteria2" {
            attributes.insert("hop_interval".to_string(), Value::from("10s"));
        }

        Self::node_outbound(node, attributes, tls_of(node))
    }

    fn node_outbound(
        node: &PickNode,
        attributes: IndexMap<String, Value>,
        tls_attributes: IndexMap<String, Value>,
    ) -> Self {
        let insecure = if node.protocol == "vless" {
            None
        } else {
            Some(node.skip_cert_verify())
        };

        Self {
            tag: node.name(),
            type_: node.protocol.clone(),
            interrupt_exist_connections: None,
            default: None,
            url: None,
            interval: None,
            tolerance: None,
            outbounds: None,
            port: node.port(),
            server: Some(node.server()),
            password: node.string("password"),
            tls: Some(OutboundTls {
                enabled: true,
                insecure,
                attributes: tls_attributes,
            }),
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

/// 构建 tls 配置, 两种来源的节点键一致。
fn tls_of(node: &PickNode) -> IndexMap<String, Value> {
    let mut tls = IndexMap::new();

    if let Some(sni) = node.string("sni").or_else(|| node.string("peer")) {
        tls.insert("server_name".to_string(), Value::String(sni));
    }

    if let Some(fingerprint) = node.map.get("fp") {
        let mut utls = serde_json::Map::new();
        utls.insert("enabled".to_string(), Value::Bool(true));
        utls.insert("fingerprint".to_string(), fingerprint.clone());
        tls.insert("utls".to_string(), Value::Object(utls));
    }

    if node.string("security").as_deref() == Some("reality") {
        let mut reality = serde_json::Map::new();
        reality.insert("enabled".to_string(), Value::Bool(true));
        if let Some(public_key) = node.map.get("pbk") {
            reality.insert("public_key".to_string(), public_key.clone());
        }
        if let Some(short_id) = node.map.get("sid") {
            reality.insert("short_id".to_string(), short_id.clone());
        }
        tls.insert("reality".to_string(), Value::Object(reality));
    }

    tls
}

/// 分享链接中的传输层参数转换为 sing-box transport。
fn transport_of(node: &PickNode, kind: &str) -> Value {
    let mut transport = serde_json::Map::new();
    transport.insert("type".to_string(), Value::String(kind.to_string()));

    if let Some(path) = node.map.get("path") {
        transport.insert("path".to_string(), path.clone());
    }

    if let Some(host) = node.map.get("host") {
        transport.insert("host".to_string(), host.clone());
    }

    if let Some(service_name) = node.map.get("serviceName") {
        transport.insert("service_name".to_string(), service_name.clone());
    }

    Value::Object(transport)
}

/// 节点中不参与出口输出的键。
fn skip_key(key: &str) -> bool {
    matches!(
        key,
        "type"
            | "name"
            | "server"
            | "port"
            | "password"
            | "allowInsecure"
            | "skip-cert-verify"
            | "peer"
            | "sni"
            | "alpn"
            | "mport"
            | "insecure"
            | "encryption"
            | "host"
            | "path"
            | "headerType"
            | "quicSecurity"
            | "serviceName"
            | "security"
            | "fp"
            | "pbk"
            | "sid"
            | "pinSHA256"
    )
}
