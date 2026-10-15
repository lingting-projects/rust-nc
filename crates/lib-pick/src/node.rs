use anyhow::Result;
use indexmap::IndexMap;
use serde_json::Value;

use crate::area::{self, Area};
use crate::value::JsonValueExt;

/// 节点名称键。
pub const KEY_NAME: &str = "name";
/// 节点服务地址键。
pub const KEY_SERVER: &str = "server";
/// 节点端口键。
pub const KEY_PORT: &str = "port";

/// 订阅节点, 内部为保留原始键顺序的 map。
#[derive(Debug, Clone, PartialEq)]
pub struct PickNode {
    /// 原始配置, 保持来源中的键顺序
    pub map: IndexMap<String, Value>,
    /// 协议所在字段名
    pub protocol_key: String,
    /// 协议值, 例如 vless、trojan、ss
    pub protocol: String,
    /// 识别出的区域
    pub area: Option<&'static Area>,
}

impl PickNode {
    /// 由原始配置与协议字段构建节点, 区域取自节点名称。
    pub fn new(map: IndexMap<String, Value>, protocol_key: impl Into<String>) -> Self {
        let protocol_key = protocol_key.into();
        let protocol = map
            .get(&protocol_key)
            .and_then(|value| value.string())
            .unwrap_or_default();
        let area = map
            .get(KEY_NAME)
            .and_then(|value| value.string())
            .and_then(|name| area::find_match(&name));

        Self {
            map,
            protocol_key,
            protocol,
            area,
        }
    }

    /// 节点名称。
    pub fn name(&self) -> String {
        self.string(KEY_NAME).unwrap_or_default()
    }

    /// 服务地址。
    pub fn server(&self) -> String {
        self.string(KEY_SERVER).unwrap_or_default()
    }

    /// 服务端口。
    pub fn port(&self) -> Option<u16> {
        self.string(KEY_PORT).and_then(|port| port.parse().ok())
    }

    /// 取字符串值。
    pub fn string(&self, key: &str) -> Option<String> {
        self.map.get(key).and_then(|value| value.string())
    }

    /// 取布尔值, 支持 `1`、`true`、`t`、`y`、`ok` 等写法。
    pub fn bool(&self, key: &str) -> Option<bool> {
        self.string(key).map(|value| is_true(&value))
    }

    /// 是否跳过证书校验。
    pub fn skip_cert_verify(&self) -> bool {
        self.bool("skip-cert-verify").unwrap_or(false)
            || self.bool("allowInsecure").unwrap_or(false)
            || self.bool("insecure").unwrap_or(false)
    }
}

/// 节点列表。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PickNodes {
    pub nodes: Vec<PickNode>,
}

impl PickNodes {
    /// 由节点集合构建。
    pub fn new(nodes: Vec<PickNode>) -> Self {
        Self { nodes }
    }

    /// 节点数量。
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// 是否没有节点。
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// 转换为 JSON 文本, 用于持久化。
    pub fn to_json(&self) -> Result<String> {
        let values: Vec<&IndexMap<String, Value>> =
            self.nodes.iter().map(|node| &node.map).collect();
        Ok(serde_json::to_string(&values)?)
    }

    /// 从 JSON 文本解析, `protocol_key` 为协议所在字段名。
    pub fn from_json(source: &str, protocol_key: &str) -> Result<Self> {
        let values: Vec<IndexMap<String, Value>> = serde_json::from_str(source)?;
        Ok(Self::new(
            values
                .into_iter()
                .map(|map| PickNode::new(map, protocol_key))
                .collect(),
        ))
    }
}

/// 字符串是否为真值。
pub fn is_true(source: &str) -> bool {
    matches!(
        source.to_lowercase().as_str(),
        "1" | "true" | "t" | "y" | "ok"
    )
}
