use lib_pick::PickNode;

use super::{Proxy, clash_keys, link_keys, tls, transport};
use crate::source::Source;

/// trojan 代理。
pub(super) struct Trojan;

impl Trojan {
    /// 协议名。
    pub(super) const PROTOCOL: &str = "trojan";

    /// clash 原生字段中的协议字段。
    const CLASH_KEYS: &[&str] = &[
        "network",
        "alpn",
        "sni",
        "client-fingerprint",
        "ws-opts",
        "grpc-opts",
    ];

    /// 由节点构建代理。
    pub(super) fn node(node: &PickNode, source: Source) -> Proxy {
        let attributes = match source {
            Source::Clash => clash_keys(node, Self::CLASH_KEYS),
            Source::Pure => {
                let mut attributes = link_keys(node, &[]);
                transport::apply(node, &mut attributes);
                tls::apply(node, &mut attributes);
                attributes
            }
        };

        Proxy::build(node, Self::PROTOCOL, attributes)
    }
}
