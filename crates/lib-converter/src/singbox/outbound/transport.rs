use lib_pick::PickNode;
use serde_json::Value;

/// 分享链接传输层类型到 sing-box 传输层类型的映射。
///
/// sing-box 只识别这些类型, 纯 tcp 不需要传输层配置。
const TRANSPORTS: &[(&str, &str)] = &[
    ("ws", "ws"),
    ("grpc", "grpc"),
    ("http", "http"),
    ("h2", "http"),
    ("httpupgrade", "httpupgrade"),
    ("quic", "quic"),
];

/// 分享链接中的传输层参数转换为 sing-box transport。
///
/// 每种传输层只接受自己的字段, 多余字段会让内核校验失败, 因此按类型分别输出。
pub(super) fn of(node: &PickNode) -> Option<Value> {
    let kind = TRANSPORTS
        .iter()
        .find(|(from, _)| *from == node.string("type").unwrap_or_default())
        .map(|(_, to)| *to)?;

    let mut transport = serde_json::Map::new();
    transport.insert("type".to_string(), Value::String(kind.to_string()));

    if let Some(path) = node.string("path").filter(|path| !path.is_empty()) {
        transport.insert("path".to_string(), Value::String(path));
    }

    let host = node.string("host").filter(|host| !host.is_empty());
    match kind {
        // ws 的 Host 只能通过请求头传递
        "ws" => {
            if let Some(host) = host {
                let mut headers = serde_json::Map::new();
                headers.insert("Host".to_string(), Value::String(host));
                transport.insert("headers".to_string(), Value::Object(headers));
            }
        }
        "http" | "httpupgrade" => {
            if let Some(host) = host {
                transport.insert("host".to_string(), Value::String(host));
            }
        }
        "grpc" => {
            if let Some(service_name) = node.string("serviceName").filter(|name| !name.is_empty()) {
                transport.insert("service_name".to_string(), Value::String(service_name));
            }
        }
        _ => {}
    }

    Some(Value::Object(transport))
}
