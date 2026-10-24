use indexmap::IndexMap;
use lib_pick::PickNode;
use lib_pick::value::json_to_yml;
use serde_yaml::Value as Yml;

/// 写入服务名与客户端指纹, 两种来源的节点键一致。
pub(super) fn apply(node: &PickNode, attributes: &mut IndexMap<String, Yml>) {
    // sni 优先, 其次为 peer
    if let Some(sni) = node.string("sni").or_else(|| node.string("peer")) {
        attributes.insert("sni".to_string(), Yml::from(sni));
    }

    if let Some(fingerprint) = node.map.get("fp") {
        attributes.insert("client-fingerprint".to_string(), json_to_yml(fingerprint));
    }
}

/// reality 参数转换为 clash 的 reality-opts。
pub(super) fn reality(node: &PickNode, attributes: &mut IndexMap<String, Yml>) {
    if node.string("security").as_deref() != Some("reality") {
        return;
    }

    let mut opts = serde_yaml::Mapping::new();
    if let Some(public_key) = node.map.get("pbk") {
        opts.insert(Yml::from("public-key"), json_to_yml(public_key));
    }
    if let Some(short_id) = node.map.get("sid") {
        opts.insert(Yml::from("short-id"), json_to_yml(short_id));
    }
    attributes.insert("reality-opts".to_string(), Yml::Mapping(opts));
}
