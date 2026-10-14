//! 订阅来源。

/// 节点来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// 由 clash 订阅转换而来, 节点键为 clash 原生字段。
    Clash,
    /// 由纯净订阅转换而来, 节点键为分享链接参数。
    Pure,
}
