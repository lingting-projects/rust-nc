use indexmap::IndexMap;
use lib_pick::PickNode;
use lib_pick::value::json_to_yml;
use serde_json::Value;
use serde_yaml::Value as Yml;

/// 分享链接传输层到 clash 网络类型的映射。
const NETWORKS: &[(&str, &str)] = &[
    ("ws", "ws"),
    ("grpc", "grpc"),
    ("http", "http"),
    ("h2", "http"),
];

/// 分享链接的传输层参数转换为 clash 的 network 与 *-opts。
///
/// clash 原生节点的网络类型已在节点字段中, 由协议自己的键表沿用。
pub(super) fn apply(node: &PickNode, attributes: &mut IndexMap<String, Yml>) {
    let Some(kind) = node.string("type") else {
        return;
    };

    let network = NETWORKS
        .iter()
        .find(|(from, _)| *from == kind)
        .map(|(_, to)| *to)
        .unwrap_or(kind.as_str());
    attributes.insert("network".to_string(), Yml::from(network));

    let mut opts = serde_yaml::Mapping::new();
    if let Some(path) = node.map.get("path") {
        opts.insert(Yml::from("path"), json_to_yml(path));
    }
    if let Some(host) = node.map.get("host") {
        opts.insert(Yml::from("headers"), headers(host));
    }
    if let Some(service_name) = node.map.get("serviceName") {
        opts.insert(Yml::from("grpc-service-name"), json_to_yml(service_name));
    }

    if opts.is_empty() {
        return;
    }

    let key = match network {
        "ws" => "ws-opts",
        "grpc" => "grpc-opts",
        "http" => "h2-opts",
        _ => return,
    };
    attributes.insert(key.to_string(), Yml::Mapping(opts));
}

/// `host` 参数转换为 clash 的 Host 请求头。
fn headers(host: &Value) -> Yml {
    let values: Vec<String> = match host {
        Value::Array(values) => values
            .iter()
            .filter_map(|value| value.as_str().map(str::to_string))
            .collect(),
        value => value
            .as_str()
            .map(|value| vec![value.to_string()])
            .unwrap_or_default(),
    };

    let mut headers = serde_yaml::Mapping::new();
    headers.insert(
        Yml::from("Host"),
        Yml::Sequence(values.into_iter().map(Yml::from).collect()),
    );
    Yml::Mapping(headers)
}
