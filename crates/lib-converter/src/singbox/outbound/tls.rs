use indexmap::IndexMap;
use lib_pick::PickNode;
use serde_json::Value;

use super::OutboundTls;

/// 构建 tls 配置, 两种来源的节点键一致。
pub(super) fn of(node: &PickNode) -> OutboundTls {
    OutboundTls {
        enabled: true,
        insecure: Some(node.skip_cert_verify()),
        attributes: attributes(node),
    }
}

/// 构建不带证书校验开关的 tls 配置。
///
/// vless 的证书校验由 reality 等参数自行决定, 出口不输出 insecure。
pub(super) fn without_insecure(node: &PickNode) -> OutboundTls {
    OutboundTls {
        enabled: true,
        insecure: None,
        attributes: attributes(node),
    }
}

/// tls 私有字段: 服务名、客户端指纹与 reality。
fn attributes(node: &PickNode) -> IndexMap<String, Value> {
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
