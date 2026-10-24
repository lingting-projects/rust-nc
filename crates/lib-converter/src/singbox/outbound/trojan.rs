use indexmap::IndexMap;
use lib_pick::PickNode;
use serde_json::Value;

use super::{Outbound, keys, pure_transport, tls};
use crate::source::Source;

/// trojan 出口。
pub(super) struct Trojan;

impl Trojan {
    /// 协议名。
    pub(super) const PROTOCOL: &str = "trojan";

    /// 可直接沿用的节点字段。
    const KEYS: &[(&str, &str)] = &[("flow", "flow")];

    /// 由节点构建出口。
    pub(super) fn node(node: &PickNode, source: Source) -> Outbound {
        let mut attributes: IndexMap<String, Value> = keys(node, Self::KEYS);
        pure_transport(node, source, &mut attributes);

        Outbound::build(node, Self::PROTOCOL, Some(tls::of(node)), attributes)
    }
}
