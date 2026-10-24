use indexmap::IndexMap;
use lib_pick::PickNode;
use serde_json::Value;

use super::{Outbound, keys, pure_transport, tls};
use crate::source::Source;

/// anytls 出口。
pub(super) struct Anytls;

impl Anytls {
    /// 协议名。
    pub(super) const PROTOCOL: &str = "anytls";

    /// 由节点构建出口。
    ///
    /// anytls 没有可直接沿用的分享链接参数。
    pub(super) fn node(node: &PickNode, source: Source) -> Outbound {
        let mut attributes: IndexMap<String, Value> = keys(node, &[]);
        pure_transport(node, source, &mut attributes);

        Outbound::build(node, Self::PROTOCOL, Some(tls::of(node)), attributes)
    }
}
