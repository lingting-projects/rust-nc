use indexmap::IndexMap;
use lib_pick::PickNode;
use lib_pick::value::json_to_yml;
use serde::Serialize;
use serde_json::Value;
use serde_yaml::Value as Yml;

use crate::source::Source;

/// 分享链接参数与 clash 字段同名, 可直接沿用。
const SAME_KEYS: &[(&str, &str)] = &[
    ("cipher", "cipher"),
    ("uuid", "uuid"),
    ("flow", "flow"),
    ("obfs", "obfs"),
    ("obfs-password", "obfs-password"),
    ("up", "up"),
    ("down", "down"),
    ("mport", "ports"),
];

/// 分享链接传输层到 clash 网络类型的映射。
const NETWORKS: &[(&str, &str)] = &[
    ("ws", "ws"),
    ("grpc", "grpc"),
    ("http", "http"),
    ("h2", "http"),
];

/// clash 代理。
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Proxy {
    pub name: String,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(rename = "skip-cert-verify")]
    pub skip_cert_verify: bool,
    pub server: String,
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(flatten)]
    pub attributes: IndexMap<String, Yml>,
}

impl Proxy {
    /// 由节点构建代理, `source` 决定节点键的翻译方式。
    pub fn node(node: &PickNode, source: Source) -> Self {
        let attributes = match source {
            Source::Clash => clash_attributes(node),
            Source::Pure => pure_attributes(node),
        };

        Self {
            name: node.name(),
            type_: if node.protocol == "shadowsocks" {
                "ss".to_string()
            } else {
                node.protocol.clone()
            },
            skip_cert_verify: node.skip_cert_verify(),
            server: node.server(),
            port: node.port().unwrap_or(0),
            password: node.string("password"),
            attributes,
        }
    }
}

/// clash 来源的节点键即 clash 原生字段, 直接沿用。
fn clash_attributes(node: &PickNode) -> IndexMap<String, Yml> {
    node.map
        .iter()
        .map(|(key, value)| (key.clone(), json_to_yml(value)))
        .collect()
}

/// 分享链接来源的节点键翻译为 clash 字段。
fn pure_attributes(node: &PickNode) -> IndexMap<String, Yml> {
    let mut attributes = IndexMap::new();

    for (from, to) in SAME_KEYS {
        if let Some(value) = node.map.get(*from) {
            attributes.insert(to.to_string(), json_to_yml(value));
        }
    }

    // sni 优先, 其次为 peer
    if let Some(sni) = node.string("sni").or_else(|| node.string("peer")) {
        attributes.insert("sni".to_string(), Yml::from(sni));
    }

    if let Some(fingerprint) = node.map.get("fp") {
        attributes.insert("client-fingerprint".to_string(), json_to_yml(fingerprint));
    }

    transport(node, &mut attributes);
    reality(node, &mut attributes);

    attributes
}

/// 传输层参数转换为 clash 的 network 与 *-opts。
fn transport(node: &PickNode, attributes: &mut IndexMap<String, Yml>) {
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

/// reality 参数转换为 clash 的 reality-opts。
fn reality(node: &PickNode, attributes: &mut IndexMap<String, Yml>) {
    if node.string("security").as_deref() != Some("reality") {
        return;
    }

    let mut opts = serde_yaml::Mapping::new();
    if let Some(public_key) = node.map.get("pbk") {
        opts.insert(Yml::from("public-key"), json_to_yml(public_key));
    }
    if let Some(short_id) = node.map.get("sid") {
        opts.insert(Yml::from("short-id"), json_to_yml(short_id));
    }
    attributes.insert("reality-opts".to_string(), Yml::Mapping(opts));
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
