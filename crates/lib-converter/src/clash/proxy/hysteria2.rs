use lib_pick::PickNode;

use super::{Proxy, clash_keys, link_keys, tls, transport};
use crate::source::Source;

/// hysteria2 代理。
pub(super) struct Hysteria2;

impl Hysteria2 {
    /// 协议名。
    pub(super) const PROTOCOL: &str = "hysteria2";

    /// 分享链接中的别名。
    pub(super) const ALIAS: &str = "hy2";

    /// clash 原生字段中的协议字段。
    const CLASH_KEYS: &[&str] = &["up", "down", "obfs", "obfs-password", "sni"];

    /// 分享链接参数中的协议字段。
    const LINK_KEYS: &[(&str, &str)] = &[
        ("up", "up"),
        ("down", "down"),
        ("obfs", "obfs"),
        ("obfs-password", "obfs-password"),
        ("mport", "ports"),
    ];

    /// 由节点构建代理。
    pub(super) fn node(node: &PickNode, source: Source) -> Proxy {
        let attributes = match source {
            Source::Clash => clash_keys(node, Self::CLASH_KEYS),
            Source::Pure => {
                let mut attributes = link_keys(node, Self::LINK_KEYS);
                transport::apply(node, &mut attributes);
                tls::apply(node, &mut attributes);
                attributes
            }
        };

        Proxy::build(node, Self::PROTOCOL, attributes)
    }
}
