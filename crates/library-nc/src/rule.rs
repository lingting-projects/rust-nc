use crate::core::{fast, AnyResult};
use crate::kernel::key_direct;
use byte_unit::rust_decimal::prelude::ToPrimitive;
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::BTreeMap;

pub enum RuleType {
    // 针对ip的规则
    Ip,
    // 针对进程的规则
    Process,
    // 其他规则
    Other,
}

impl RuleType {
    pub const fn name(self) -> &'static str {
        match self {
            RuleType::Ip => "ip",
            RuleType::Process => "process",
            RuleType::Other => "other",
        }
    }
}

pub struct Rule {
    pub path: String,
    pub rule_type: RuleType,
    pub remote: bool,
}

impl Rule {
    pub fn from_local(rule_type: RuleType, path: String) -> Self {
        Self {
            rule_type,
            path,
            remote: false,
        }
    }

    pub fn from_remote(rule_type: RuleType, url: String) -> Self {
        let path = fast(&url);
        Self {
            rule_type,
            path,
            remote: true,
        }
    }

    pub fn sing_box(&self, tag: &str) -> SingBoxRule {
        let tag = tag.into();
        let format = if self.path.ends_with("srs") {
            "binary"
        } else {
            "source"
        }
        .into();

        if self.remote {
            SingBoxRule {
                tag,
                type_: "remote".into(),
                format,
                url: Some(self.path.to_string()),
                path: None,
                download_detour: Some(key_direct.into()),
                update_interval: Some("1d".into()),
            }
        } else {
            SingBoxRule {
                tag,
                type_: "local".into(),
                format,
                url: None,
                path: Some(self.path.to_string()),
                download_detour: Some(key_direct.into()),
                update_interval: Some("1d".into()),
            }
        }
    }

    pub fn clash(&self, tag: &str) -> ClashRule {
        let name = tag.into();
        let format = "yaml".into();
        let behavior = "classical".into();

        if self.remote {
            ClashRule {
                name,
                format,
                behavior,
                type_: "http".into(),
                url: Some(self.path.to_string()),
                path: format!("./rules/{}.yml", tag),
                interval: Some(86400),
            }
        } else {
            ClashRule {
                name,
                format,
                behavior,
                type_: "file".into(),
                url: None,
                path: self.path.to_string(),
                interval: None,
            }
        }
    }
}

#[derive(Serialize)]
pub struct SingBoxRule {
    pub tag: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_detour: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_interval: Option<String>,
}

pub struct SinBoxJsonRule {
    pub type_: RuleType,
    pub json: String,
    pub count: u64,
}

impl SinBoxJsonRule {
    pub fn json_classical(raw: &str) -> AnyResult<Vec<SinBoxJsonRule>> {
        Self::_json_classical(raw, false)
    }

    pub fn json_classical_process(raw: &str) -> AnyResult<Vec<SinBoxJsonRule>> {
        Self::_json_classical(raw, true)
    }

    fn _json_classical(raw: &str, with_process: bool) -> AnyResult<Vec<SinBoxJsonRule>> {
        let mut process = Vec::new();
        let mut ip = Vec::new();
        let mut other = BTreeMap::new();

        for raw_line in raw.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with("#") || !line.contains(",") {
                continue;
            }

            let split: Vec<&str> = line.splitn(2, ",").collect();

            if split.len() != 2 {
                continue;
            }
            let (_raw_type, _value) = (split[0].trim(), split[1].trim());

            if _raw_type.is_empty() || _value.is_empty() {
                continue;
            }

            let _type = match _raw_type.to_lowercase().as_str() {
                "dst-port" => "port".into(),
                _r => _r.replace("-", "_"),
            };

            match _raw_type.to_lowercase().as_str() {
                "ip_cidr" => ip.push(_value),
                "process" => process.push(_value),
                _r => {
                    let _type = if _r == "dst-port" {
                        "port".to_string()
                    } else {
                        _r.replace("-", "_")
                    };

                    match other.get_mut(&_type) {
                        None => {
                            let vec = vec![_value];
                            other.insert(_type, vec.to_owned());
                        }
                        Some(vec) => {
                            vec.push(_value);
                        }
                    }
                }
            }
        }

        let mut vec = vec![];

        if !ip.is_empty() {
            let mut _ip = Map::new();
            _ip.insert(
                "ip_cidr".into(),
                Value::Array(ip.iter().map(|s| Value::String((*s).into())).collect()),
            );
            let mut _json = BTreeMap::new();
            _json.insert("version", Value::from(2));
            _json.insert("rules", Value::Object(_ip));
            vec.push(SinBoxJsonRule {
                type_: RuleType::Ip,
                json: serde_json::to_string(&_json)?,
                count: ip.len().to_u64().unwrap(),
            });
        }

        if !process.is_empty() {
            let _type = "process";
            if with_process {
                let mut _process = Map::new();
                _process.insert(
                    _type.into(),
                    Value::Array(process.iter().map(|s| Value::String((*s).into())).collect()),
                );
                let mut _json = BTreeMap::new();
                _json.insert("version", Value::from(2));
                _json.insert("rules", Value::Object(_process));
                vec.push(SinBoxJsonRule {
                    type_: RuleType::Process,
                    json: serde_json::to_string(&_json)?,
                    count: process.len().to_u64().unwrap(),
                });
            } else {
                other.insert(_type.to_string(), process);
            }
        }

        if !other.is_empty() {
            let mut _other = Map::new();
            let mut count: u64 = 0;
            for (_k, _vs) in other {
                count = count + _vs.len().to_u64().unwrap();
                let k = _k.to_string();
                let vs = Value::Array(_vs.iter().map(|s| Value::from(*s)).collect());
                _other.insert(k, vs);
            }
            let mut _json = BTreeMap::new();
            _json.insert("version", Value::from(2));
            _json.insert("rules", Value::Object(_other));
            vec.push(SinBoxJsonRule {
                type_: RuleType::Other,
                json: serde_json::to_string(&_json)?,
                count,
            });
        }

        Ok(vec)
    }
}

#[derive(Serialize)]
pub struct ClashRule {
    #[serde(skip)]
    pub name: String,
    pub format: String,
    pub behavior: String,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<u32>,
}
