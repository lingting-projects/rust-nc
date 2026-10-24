use lib_pick::PickNode;

use super::{Proxy, clash_keys, link_keys, tls, transport};
use crate::source::Source;

/// vless 代理。
pub(super) struct Vless;

impl Vless {
    /// 协议名。
    pub(super) const PROTOCOL: &str = "vless";

    /// clash 原生字段中的协议字段。
    const CLASH_KEYS: &[&str] = &[
        "uuid",
        "flow",
        "packet-encoding",
        "network",
        "tls",
        "alpn",
        "servername",
        "ws-opts",
        "grpc-opts",
        "h2-opts",
        "http-opts",
        "reality-opts",
        "client-fingerprint",
    ];

    /// 分享链接参数中的协议字段。
    const LINK_KEYS: &[(&str, &str)] = &[("uuid", "uuid"), ("flow", "flow")];

    /// 由节点构建代理。
    pub(super) fn node(node: &PickNode, source: Source) -> Proxy {
        let attributes = match source {
            Source::Clash => clash_keys(node, Self::CLASH_KEYS),
            Source::Pure => {
                let mut attributes = link_keys(node, Self::LINK_KEYS);
                transport::apply(node, &mut attributes);
                tls::apply(node, &mut attributes);
                tls::reality(node, &mut attributes);
                attributes
            }
        };

        Proxy::build(node, Self::PROTOCOL, attributes)
    }
}
