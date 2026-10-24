use indexmap::IndexMap;
use lib_pick::PickNode;
use serde_json::Value;

use super::{Outbound, keys, pure_transport, tls};
use crate::source::Source;

/// shadowsocks 出口。
pub(super) struct ShadowSocks;

impl ShadowSocks {
    /// 协议名。
    pub(super) const PROTOCOL: &str = "shadowsocks";

    /// 分享链接中的别名。
    pub(super) const ALIAS: &str = "ss";

    /// 可直接沿用的节点字段。
    const KEYS: &[(&str, &str)] = &[
        ("method", "method"),
        ("plugin", "plugin"),
        ("plugin_opts", "plugin_opts"),
        ("udp_over_tcp", "udp_over_tcp"),
    ];

    /// 由节点构建出口。
    ///
    /// 两种来源的协议名不同, 出口统一使用内核识别的 `shadowsocks`。
    pub(super) fn node(node: &PickNode, source: Source) -> Outbound {
        let mut attributes: IndexMap<String, Value> = keys(node, Self::KEYS);
        pure_transport(node, source, &mut attributes);

        Outbound::build(node, Self::PROTOCOL, Some(tls::of(node)), attributes)
    }
}
