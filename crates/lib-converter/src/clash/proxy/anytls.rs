use lib_pick::PickNode;

use super::{Proxy, clash_keys, link_keys, tls};
use crate::source::Source;

/// anytls 代理。
pub(super) struct Anytls;

impl Anytls {
    /// 协议名。
    pub(super) const PROTOCOL: &str = "anytls";

    /// clash 原生字段中的协议字段。
    const CLASH_KEYS: &[&str] = &["sni", "alpn", "client-fingerprint"];

    /// 由节点构建代理。
    ///
    /// anytls 没有可直接沿用的分享链接参数。
    pub(super) fn node(node: &PickNode, source: Source) -> Proxy {
        let attributes = match source {
            Source::Clash => clash_keys(node, Self::CLASH_KEYS),
            Source::Pure => {
                let mut attributes = link_keys(node, &[]);
                tls::apply(node, &mut attributes);
                attributes
            }
        };

        Proxy::build(node, Self::PROTOCOL, attributes)
    }
}
