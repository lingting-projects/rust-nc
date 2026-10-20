//! 内核配置使用的网段。
//!
//! 这些取值由转换层直接写入内核配置，界面需要展示同一份口径，
//! 因此在此集中公开；`constants` 仍保持 crate 私有。

pub use crate::constants::{
    FAKE_IPV4, FAKE_IPV6, INNER_IPV4, INNER_IPV6, LOOPBACK_IPV4, LOOPBACK_IPV6, ROUTE_IPV4,
    ROUTE_IPV6, VIRTUAL_IPV4, VIRTUAL_IPV6,
};
