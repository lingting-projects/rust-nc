use indexmap::IndexMap;
use lib_pick::PickNode;
use serde_json::Value;

use super::{Outbound, keys, pure_transport, tls};
use crate::source::Source;

/// vless 出口。
pub(super) struct Vless;

impl Vless {
    /// 协议名。
    pub(super) const PROTOCOL: &str = "vless";

    /// 可直接沿用的节点字段。
    const KEYS: &[(&str, &str)] = &[
        ("uuid", "uuid"),
        ("flow", "flow"),
        ("packet_encoding", "packet_encoding"),
    ];

    /// 由节点构建出口。
    pub(super) fn node(node: &PickNode, source: Source) -> Outbound {
        let mut attributes: IndexMap<String, Value> = keys(node, Self::KEYS);
        pure_transport(node, source, &mut attributes);

        // vless 的证书校验由 reality 等参数决定, 出口不输出 insecure
        Outbound::build(
            node,
            Self::PROTOCOL,
            Some(tls::without_insecure(node)),
            attributes,
        )
    }
}
