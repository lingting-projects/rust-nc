use indexmap::IndexMap;
use lib_pick::PickNode;
use serde_json::Value;

use super::{Outbound, keys, pure_transport, tls};
use crate::source::Source;

/// hysteria2 出口。
pub(super) struct Hysteria2;

impl Hysteria2 {
    /// 协议名。
    pub(super) const PROTOCOL: &str = "hysteria2";

    /// 分享链接中的别名。
    pub(super) const ALIAS: &str = "hy2";

    /// 可直接沿用的节点字段。
    const KEYS: &[(&str, &str)] = &[
        ("up", "up"),
        ("down", "down"),
        ("obfs", "obfs"),
        ("obfs-password", "obfs-password"),
    ];

    /// 由节点构建出口。
    pub(super) fn node(node: &PickNode, source: Source) -> Outbound {
        let mut attributes: IndexMap<String, Value> = keys(node, Self::KEYS);
        pure_transport(node, source, &mut attributes);
        attributes.insert("hop_interval".to_string(), Value::from("10s"));

        Outbound::build(node, Self::PROTOCOL, Some(tls::of(node)), attributes)
    }
}
