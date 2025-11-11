use crate::area;
use crate::area::{find, Area};
use crate::core::{base64_decode, NcError, PREFIX_EXPIRE, PREFIX_REMAIN_TRAFFIC};
use crate::http::url_decode;
use byte_unit::rust_decimal::prelude::ToPrimitive;
use indexmap::IndexMap;
use library_core::boolean::is_true;
use library_core::core::AnyResult;
use library_core::data_size;
use library_core::data_size::DataSize;
use library_core::json::JsonValueExt;
use library_core::yml::YmlValueExt;
use serde::de::{Error, MapAccess, Visitor};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::fmt;
use time::macros::format_description;
use time::PrimitiveDateTime;
#[cfg(feature = "wrangler")]
use worker::{console_error, console_warn};

#[derive(Debug, Default)]
pub struct Subscribe {
    /// 已使用下行流量. 单位: bytes
    pub download: Option<u64>,
    /// 已使用上行流量. 单位: bytes
    pub upload: Option<u64>,
    /// 最多流量. 单位: bytes
    pub max: Option<u64>,
    /// 过期时间. 毫秒级别时间戳
    pub expire: Option<u64>,
    /// 拥有的节点
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
                            if let Ok(size) = DataSize::parse(b) {
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
                let map: IndexMap<_, _> = info
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
        let used = DataSize::of_bytes(download + upload);
        let expire = self
            .expire
            .map(|u| {
                if u < 1 {
                    None
                } else {
                    Some(format!("{:.2}", u / 1000))
                }
            })
            .flatten()
            .unwrap_or("0".into());
        Some(format!(
            "download={}; upload={}; used={}; total={}; expire={}",
            download, upload, used, max, expire
        ))
    }
}

#[derive(Debug, Default)]
pub struct SubscribeNode {
    pub node_type: String,
    pub name: String,
    pub server: String,
    pub port: Option<u16>,
    pub password: Option<String>,
    pub area: Option<&'static Area>,
    pub attribute: IndexMap<String, Value>,
}

impl SubscribeNode {
    pub const VLESS_UUID_KEY: &'static str = "uuid";

    /// 从文本解析多个节点
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
            } else if line.starts_with("vless://") {
                Self::from_vless_text(line)
            } else if line.starts_with("hysteria2://") {
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

    /// 从 ShadowSocks 格式文本解析节点
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

        let port = port_str.parse::<u16>()?;
        let name = url_decode(name_encoded)?;
        let area = area::find_match(&name);

        let mut attribute = IndexMap::new();
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

    /// 从Trojan格式文本解析节点
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
            host_parts[1].replace("/", "").parse::<u16>()?
        } else {
            443
        };

        let (param_part, name_encoded) = if rest_part.contains('#') {
            let parts: Vec<&str> = rest_part.splitn(2, '#').collect();
            (parts[0], parts[1])
        } else {
            ("", rest_part)
        };

        let attribute = Self::_params(param_part);

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

    /// 从 Vless 格式文本解析
    pub fn from_vless_text(source: &str) -> AnyResult<Option<Self>> {
        let source = source.trim();
        if source.is_empty() {
            return Ok(None);
        }

        let parts: Vec<&str> = source.splitn(2, "://").collect();
        let (type_part, rest) = (parts[0], parts[1]);

        let parts: Vec<&str> = rest.splitn(2, '@').collect();
        let (uuid, rest) = (parts[0], parts[1]);

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
            host_parts[1].parse::<u16>()?
        } else {
            443
        };

        let (param_part, name_encoded) = if rest_part.contains('#') {
            let parts: Vec<&str> = rest_part.splitn(2, '#').collect();
            (parts[0], parts[1])
        } else {
            ("", rest_part)
        };

        let mut attribute = Self::_params(param_part);
        attribute.insert(
            Self::VLESS_UUID_KEY.to_string(),
            Value::String(uuid.to_string()),
        );

        let name = url_decode(name_encoded)?;
        let area = area::find_match(&name);

        let node = Self {
            node_type: type_part.trim().to_string(),
            name,
            server,
            port: Some(port),
            password: None,
            area,
            attribute,
        };
        Ok(Some(node))
    }

    /// 从任意格式解析节点
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

    fn _params(source: &str) -> IndexMap<String, Value> {
        let mut map = IndexMap::new();
        if source.is_empty() {
            return map;
        }

        for param in source.split('&') {
            if param.is_empty() {
                continue;
            }

            let parts: Vec<&str> = param.splitn(2, '=').collect();
            if parts.len() == 2 {
                let (key, value) = (parts[0], parts[1]);
                map.insert(key.to_string(), Value::String(value.to_string()));
            }
        }
        map
    }

    pub fn attr_vec(&self, key: &str) -> Option<Vec<String>> {
        let v = self.attribute.get(key)?;
        let array = v.as_array()?;
        array.iter().map(|i| i.string()).collect()
    }

    pub fn attr_string(&self, key: &str) -> Option<String> {
        let v = self.attribute.get(key)?;
        v.string()
    }

    pub fn attr_bool(&self, key: &str) -> Option<bool> {
        self.attr_string(key).map(|s| is_true(&s))
    }

    pub fn disable_ssl(&self) -> bool {
        if self.attr_bool("skip-cert-verify").unwrap_or(false) {
            return true;
        }
        if self.attr_bool("allowInsecure").unwrap_or(false) {
            return true;
        }
        if self.attr_bool("insecure").unwrap_or(false) {
            return true;
        }
        false
    }
}

/// 实现 SubscribeNode 的序列化
impl Serialize for SubscribeNode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 计算需要序列化的字段数量（排除为 None 的 Option 字段）
        let mut field_count = 3;
        // 必选字段: node_type, name, server
        if self.port.is_some() {
            field_count += 1;
        }
        if self.password.is_some() {
            field_count += 1;
        }
        if self.area.is_some() {
            field_count += 1;
        }
        if !self.attribute.is_empty() {
            field_count += 1;
        }

        let mut state = serializer.serialize_struct("SubscribeNode", field_count)?;
        state.serialize_field("node_type", &self.node_type)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("server", &self.server)?;

        if let Some(port) = &self.port {
            state.serialize_field("port", port)?;
        }

        if let Some(password) = &self.password {
            state.serialize_field("password", password)?;
        }

        // 特殊处理 area 字段，只序列化 code
        if let Some(area) = &self.area {
            state.serialize_field("area", &area.code)?;
        }

        if !self.attribute.is_empty() {
            state.serialize_field("attribute", &self.attribute)?;
        }

        state.end()
    }
}

/// 实现 SubscribeNode 的反序列化
struct SubscribeNodeVisitor;

impl<'de> Visitor<'de> for SubscribeNodeVisitor {
    type Value = SubscribeNode;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("struct SubscribeNode")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut node_type = None;
        let mut name = None;
        let mut server = None;
        let mut port = None;
        let mut password = None;
        let mut area_code: Option<&str> = None; // 存储 area 的 code 字符串
        let mut attribute = IndexMap::new();

        // 处理所有键值对
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "node_type" => node_type = Some(map.next_value()?),
                "name" => name = Some(map.next_value()?),
                "server" => server = Some(map.next_value()?),
                "port" => port = Some(map.next_value()?),
                "password" => password = Some(map.next_value()?),
                // 存储 code 字符串
                "area" => area_code = Some(map.next_value()?),
                "attribute" => attribute = map.next_value()?,
                _ => { /* 忽略未知字段 */ }
            }
        }

        // 验证必需字段
        let node_type = node_type.ok_or_else(|| Error::missing_field("node_type"))?;
        let name = name.ok_or_else(|| Error::missing_field("name"))?;
        let server = server.ok_or_else(|| Error::missing_field("server"))?;

        // 通过 code 查找 area
        let area = match area_code {
            Some(code) => find(&code),
            None => None,
        };

        Ok(SubscribeNode {
            node_type,
            name,
            server,
            port,
            password,
            area,
            attribute,
        })
    }
}

impl<'de> Deserialize<'de> for SubscribeNode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_struct(
            "SubscribeNode",
            &[
                "node_type",
                "name",
                "server",
                "port",
                "password",
                "area",
                "attribute",
            ],
            SubscribeNodeVisitor,
        )
    }
}

pub const HEADER_INFO: &str = "Subscription-Userinfo";
