use crate::area::Area;
use crate::core::{base64_decode, AnyResult, NcError, PREFIX_EXPIRE, PREFIX_REMAIN_TRAFFIC};
use crate::http::url_decode;
use crate::{area, data_size};
use byte_unit::rust_decimal::prelude::ToPrimitive;
use std::collections::HashMap;
use serde_json::Value;
use time::macros::format_description;
use time::PrimitiveDateTime;
use worker::{console_error, console_warn};

pub struct Subscribe {
    // 已使用下行流量. 单位: bytes
    pub download: Option<u64>,
    // 已使用上行流量. 单位: bytes
    pub upload: Option<u64>,
    // 最多流量. 单位: bytes
    pub max: Option<u64>,
    // 过期时间. 毫秒级别时间戳
    pub expire: Option<u64>,
    // 拥有的节点
    pub nodes: Vec<SubscribeNode>,
}

impl Subscribe {
    pub fn resolve(input: &str, header_user_info: Option<String>) -> AnyResult<Self> {
        let nodes = SubscribeNode::resolve(input)?;

        let mut download: Option<u64> = None;
        let mut upload: Option<u64> = None;
        let mut max: Option<u64> = None;
        let mut expire: Option<u64> = None;

        match header_user_info {
            None => {
                nodes.iter().for_each(|node| {
                    if node.area.is_some() {
                        return;
                    }
                    let name = &node.name;
                    if name.is_empty() {
                        return;
                    }

                    // 剩余流量
                    let po = PREFIX_REMAIN_TRAFFIC.iter().find(|p| name.starts_with(*p));
                    if po.is_some() {
                        let p = po.unwrap();
                        let bo = name.strip_prefix(p);
                        if bo.is_some() {
                            let b = bo.unwrap();
                            if let Ok(size) = data_size::DataSize::parse(b) {
                                max = Some(size.bytes)
                            }
                        }
                        return;
                    }
                    let po = PREFIX_EXPIRE.iter().find(|p| name.starts_with(*p));
                    if po.is_some() {
                        let p = po.unwrap();
                        let bo = name.strip_prefix(p);
                        if bo.is_some() {
                            let b = bo.unwrap();

                            let format = format_description!(
                                "[year]-[month]-[day] [hour]:[minute]:[second]"
                            );

                            if let Ok(time) = PrimitiveDateTime::parse(b, format) {
                                expire = Some(time.millisecond().to_u64().unwrap())
                            }
                        }
                        return;
                    }
                })
            }
            Some(info) => {
                let map: HashMap<_, _> = info
                    .split("; ")
                    .filter_map(|item| {
                        let parts: Vec<_> = item.splitn(2, '=').collect();
                        if parts.len() == 2 {
                            Some((parts[0].to_string(), parts[1].to_string()))
                        } else {
                            None
                        }
                    })
                    .collect();

                download = map.get("download").and_then(|s| s.parse::<u64>().ok());
                upload = map.get("upload").and_then(|s| s.parse::<u64>().ok());
                let total = map
                    .get("total")
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0);
                let expire_seconds = map
                    .get("expire")
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0);

                max = Some(total);
                expire = Some(expire_seconds * 1000)
            }
        }

        Ok(Self {
            download,
            upload,
            max,
            expire,
            nodes,
        })
    }

    pub fn info(&self) -> Option<String> {
        if self.download.is_none()
            && self.upload.is_none()
            && self.max.is_none()
            && self.expire.is_none()
        {
            return None;
        }

        let download = self.download.unwrap_or(0);
        let upload = self.upload.unwrap_or(0);
        let max = self.max.unwrap_or(0);
        let expire = self.expire.unwrap_or(0);
        let expire_str = &expire.to_string();
        Some(format!(
            "download={};upload={};total={};expire={};",
            download,
            upload,
            max,
            if expire < 1 { "-" } else { expire_str }
        ))
    }
}

#[derive(Debug)]
pub struct SubscribeNode {
    pub node_type: String,
    pub name: String,
    pub server: String,
    pub port: Option<i32>,
    pub password: Option<String>,
    pub area: Option<&'static Area>,
    pub attribute: HashMap<String, Value>,
}

impl SubscribeNode {
    // 从YAML字符串解析多个节点
    pub fn from_yaml(yaml_str: &str) -> AnyResult<Vec<Self>> {
        let load: HashMap<String, Value> = serde_yaml::from_str(yaml_str)?;
        let proxies = load
            .get("proxies")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        let mut nodes = Vec::new();

        for proxy in proxies {
            if let Some(proxy_map) = proxy.as_object() {
                let mut node_type = String::new();
                let mut name = String::new();
                let mut server = String::new();
                let mut port: Option<i32> = None;
                let mut password: Option<String> = None;
                let mut attribute = HashMap::new();
                let mut area: Option<&'static Area> = None;

                for (key, value) in proxy_map {
                    match key.as_str() {
                        "name" => {
                            name = value.as_str().unwrap_or("").trim().to_string();
                            area = area::find_match(&name);
                        }
                        "type" => node_type = value.as_str().unwrap_or("").trim().to_string(),
                        "server" => server = value.as_str().unwrap_or("").trim().to_string(),
                        "port" => {
                            if let Some(port_str) = value.as_str() {
                                port = port_str.parse().ok();
                            }
                        }
                        "password" => password = value.as_str().map(|s| s.trim().to_string()),
                        _ => {
                            attribute.insert(key.clone(), value.clone());
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

    // 从文本解析多个节点
    pub fn from_text(text: &str) -> Vec<Self> {
        let mut nodes = Vec::new();

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let r: AnyResult<_> = if line.starts_with("ss://") || line.starts_with("shadowsocks://")
            {
                Self::from_shadow_socks_text(line)
            } else if line.starts_with("trojan://") {
                Self::from_trojan_text(line)
            } else {
                Err(Box::new(NcError::UnsupportedSource))
            };

            match r {
                Ok(o) => match o {
                    None => {
                        #[cfg(feature = "binary")]
                        log::warn!("解析结果为空! {}", line);
                        #[cfg(feature = "wrangler")]
                        console_warn!("解析结果为空! {}", line);
                    }
                    Some(node) => {
                        nodes.push(node);
                    }
                },
                Err(e) => {
                    #[cfg(feature = "binary")]
                    log::error!("解析异常! {}; {}", line, e);
                    #[cfg(feature = "wrangler")]
                    console_error!("解析异常! {}; {}", line, e);
                }
            }
        }

        nodes
    }

    // 从 ShadowSocks 格式文本解析节点
    pub fn from_shadow_socks_text(source: &str) -> AnyResult<Option<Self>> {
        let source = source.trim();
        if source.is_empty() {
            return Ok(None);
        }

        let parts: Vec<&str> = source.splitn(2, "://").collect();
        let (type_part, rest) = (parts[0], parts[1]);

        let parts: Vec<&str> = rest.splitn(2, '@').collect();
        let (secret_base64, config_part) = (parts[0], parts[1]);

        let secret_decoded = base64_decode(secret_base64)?;
        let parts: Vec<&str> = secret_decoded.splitn(2, ':').collect();
        let (cipher, password) = (parts[0], parts[1]);

        let parts: Vec<&str> = config_part.splitn(2, ':').collect();
        let (server, rest) = (parts[0], parts[1]);

        let parts: Vec<&str> = rest.splitn(2, '#').collect();
        let (port_str, name_encoded) = (parts[0], parts[1]);

        let port = port_str.parse::<i32>()?;
        let name = url_decode(name_encoded)?;
        let area = area::find_match(&name);

        let mut attribute = HashMap::new();
        attribute.insert("cipher".to_string(), Value::String(cipher.to_string()));
        attribute.insert("udp".to_string(), Value::Bool(false));

        let node = Self {
            node_type: type_part.trim().to_string(),
            name,
            server: server.trim().to_string(),
            port: Some(port),
            password: Some(password.trim().to_string()),
            area,
            attribute,
        };
        Ok(Some(node))
    }

    // 从Trojan格式文本解析节点
    pub fn from_trojan_text(source: &str) -> AnyResult<Option<Self>> {
        let source = source.trim();
        if source.is_empty() {
            return Ok(None);
        }

        let parts: Vec<&str> = source.splitn(2, "://").collect();
        let (type_part, rest) = (parts[0], parts[1]);

        let parts: Vec<&str> = rest.splitn(2, '@').collect();
        let (password, rest) = (parts[0], parts[1]);

        let (host_part, rest_part) = if rest.contains('?') {
            let parts: Vec<&str> = rest.splitn(2, '?').collect();
            (parts[0], parts[1])
        } else {
            let parts: Vec<&str> = rest.splitn(2, '#').collect();
            (parts[0], parts[1])
        };

        let host_parts: Vec<&str> = host_part.split(':').collect();
        let server = host_parts[0].trim().to_string();
        let port = if host_parts.len() > 1 {
            host_parts[1].parse::<i32>()?
        } else {
            443
        };

        let (param_part, name_encoded) = if rest_part.contains('#') {
            let parts: Vec<&str> = rest_part.splitn(2, '#').collect();
            (parts[0], parts[1])
        } else {
            ("", rest_part)
        };

        let mut attribute = HashMap::new();

        for param in param_part.split('&') {
            if param.is_empty() {
                continue;
            }

            let parts: Vec<&str> = param.splitn(2, '=').collect();
            if parts.len() == 2 {
                let (key, value) = (parts[0], parts[1]);
                attribute.insert(key.to_string(), Value::String(value.to_string()));
            }
        }

        let name = url_decode(name_encoded)?;
        let area = area::find_match(&name);

        let node = Self {
            node_type: type_part.trim().to_string(),
            name,
            server,
            port: Some(port),
            password: Some(password.trim().to_string()),
            area,
            attribute,
        };
        Ok(Some(node))
    }

    // 从任意格式解析节点
    pub fn resolve(input: &str) -> AnyResult<Vec<Self>> {
        let input = input.trim();
        if input.is_empty() {
            return Ok(Vec::new());
        }

        let lines: Vec<&str> = input.lines().collect();

        // 如果只有一行，尝试Base64解码
        if lines.len() == 1 {
            if let Ok(decoded) = base64_decode(lines[0]) {
                return Self::resolve(&decoded);
            }
        }

        // 优先尝试YAML解析
        let nodes = match Self::from_yaml(input) {
            Ok(nodes) => nodes,
            Err(_) => Self::from_text(input),
        };

        Ok(nodes)
    }
}
