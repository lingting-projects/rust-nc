//! 请求与响应头的转发工具。
//!
//! 逐跳头只在单次连接内有效, 转发给目标前必须移除。

use std::collections::HashMap;

use lib_core::MultiStringValue;
use worker::Headers;

/// 逐跳头, 转发时必须移除。
const HOP_HEADERS: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
];

/// 响应头中不应转发的头, 内容已由客户端解压并重新计算长度。
const RESPONSE_SKIP: &[&str] = &[
    "content-encoding",
    "content-length",
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
];

/// 是否为逐跳头。
fn is_hop(name: &str) -> bool {
    HOP_HEADERS.contains(&name.to_ascii_lowercase().as_str())
}

/// 请求头是否应转发给目标。
///
/// `host` 指向当前来源, 转发后会导致目标校验失败, 因此一并移除。
pub fn forwardable(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name != "host" && !is_hop(&name)
}

/// 响应头是否应返回给来源。
pub fn response_forwardable(name: &str) -> bool {
    !RESPONSE_SKIP.contains(&name.to_ascii_lowercase().as_str())
}

/// 将来源请求的非逐跳头转换为 [`MultiStringValue`]。
pub fn forward_headers(headers: &Headers) -> MultiStringValue {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for (name, value) in headers.entries() {
        if !forwardable(&name) {
            continue;
        }
        map.entry(name.to_ascii_lowercase())
            .or_default()
            .push(value);
    }

    MultiStringValue::create(true, map)
}
