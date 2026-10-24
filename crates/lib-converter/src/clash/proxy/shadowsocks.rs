use lib_pick::PickNode;

use super::{Proxy, clash_keys, link_keys, tls, transport};
use crate::source::Source;

/// shadowsocks 代理。
pub(super) struct ShadowSocks;

impl ShadowSocks {
    /// 协议名。
    pub(super) const PROTOCOL: &str = "shadowsocks";

    /// 分享链接中的别名。
    pub(super) const ALIAS: &str = "ss";

    /// clash 代理中的类型名, 两种来源统一输出 `ss`。
    const TYPE: &str = "ss";

    /// clash 原生字段中的协议字段。
    const CLASH_KEYS: &[&str] = &["cipher", "plugin", "plugin-opts", "udp-over-tcp"];

    /// 分享链接参数中的协议字段。
    const LINK_KEYS: &[(&str, &str)] = &[("cipher", "cipher")];

    /// 由节点构建代理。
    pub(super) fn node(node: &PickNode, source: Source) -> Proxy {
        let attributes = match source {
            Source::Clash => clash_keys(node, Self::CLASH_KEYS),
            Source::Pure => {
                let mut attributes = link_keys(node, Self::LINK_KEYS);
                tls::apply(node, &mut attributes);
                attributes
            }
        };

        Proxy::build(node, Self::TYPE, attributes)
    }
}
