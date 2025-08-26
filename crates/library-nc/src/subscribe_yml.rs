use crate::area;
use crate::area::Area;
use crate::subscribe::SubscribeNode;
use indexmap::IndexMap;
use library_core::core::AnyResult;
use library_core::yml::YmlValueExt;
use serde_yaml::Value;

impl SubscribeNode {
    /// 从YAML字符串解析多个节点
    pub fn from_yaml(yaml_str: &str) -> AnyResult<Vec<Self>> {
        let load: IndexMap<String, Value> = serde_yaml::from_str(yaml_str)?;
        let proxies = load
            .get("proxies")
            .and_then(Value::as_sequence)
            .cloned()
            .unwrap_or_default();

        let mut nodes = Vec::new();

        for proxy in proxies {
            if let Some(proxy_map) = proxy.as_mapping() {
                let mut node_type = String::new();
                let mut name = String::new();
                let mut server = String::new();
                let mut port: Option<u16> = None;
                let mut password: Option<String> = None;
                let mut attribute = IndexMap::new();
                let mut area: Option<&'static Area> = None;

                for (_k, value) in proxy_map {
                    let key = _k.string_empty();
                    match key.as_str() {
                        "name" => {
                            name = value.string_empty();
                            area = area::find_match(&name);
                        }
                        "type" => node_type = value.string_empty(),
                        "server" => server = value.string_empty(),
                        "port" => {
                            if let Some(port_str) = value.string() {
                                port = port_str.parse().ok();
                            }
                        }
                        "password" => password = value.as_str().map(|s| s.trim().to_string()),
                        _ => {
                            attribute.insert(key, value.json());
                        }
                    }
                }

                let node = Self {
                    node_type: node_type.trim().to_string(),
                    name: name.clone(),
                    server: server.trim().to_string(),
                    port,
                    password: password.as_ref().map(|s| s.trim().to_string()),
                    area,
                    attribute: attribute.clone(),
                };
                nodes.push(node);
            }
        }

        Ok(nodes)
    }
}
