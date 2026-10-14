//! JSON 与 YAML 值的取值扩展。

use serde_json::Value;
use serde_yaml::Value as Yml;

/// 将任意标量值转为字符串。
pub trait JsonValueExt {
    fn string(&self) -> Option<String>;
}

impl JsonValueExt for Value {
    fn string(&self) -> Option<String> {
        match self {
            Value::Null => None,
            Value::Bool(value) => Some(value.to_string()),
            Value::Number(value) => Some(value.to_string()),
            Value::String(value) => Some(value.clone()),
            Value::Array(values) => serde_json::to_string(values).ok(),
            Value::Object(values) => serde_json::to_string(values).ok(),
        }
    }
}

/// YAML 值转 JSON 值。
pub fn yml_to_json(value: &Yml) -> Value {
    match value {
        Yml::Null => Value::Null,
        Yml::Bool(value) => Value::Bool(*value),
        Yml::Number(value) => {
            if let Some(number) = value.as_i64() {
                return Value::from(number);
            }
            if let Some(number) = value.as_u64() {
                return Value::from(number);
            }
            value.as_f64().map(Value::from).unwrap_or(Value::Null)
        }
        Yml::String(value) => Value::String(value.clone()),
        Yml::Sequence(values) => Value::Array(values.iter().map(yml_to_json).collect()),
        Yml::Mapping(mapping) => {
            let mut map = serde_json::Map::new();
            for (key, value) in mapping {
                let key = match key {
                    Yml::String(key) => key.clone(),
                    key => match yml_to_json(key) {
                        Value::String(key) => key,
                        _ => continue,
                    },
                };
                map.insert(key, yml_to_json(value));
            }
            Value::Object(map)
        }
        _ => Value::Null,
    }
}

/// JSON 值转 YAML 值。
pub fn json_to_yml(value: &Value) -> Yml {
    match value {
        Value::Null => Yml::Null,
        Value::Bool(value) => Yml::Bool(*value),
        Value::Number(value) => {
            if let Some(number) = value.as_i64() {
                return Yml::from(number);
            }
            if let Some(number) = value.as_u64() {
                return Yml::from(number);
            }
            value.as_f64().map(Yml::from).unwrap_or(Yml::Null)
        }
        Value::String(value) => Yml::from(value.clone()),
        Value::Array(values) => Yml::Sequence(values.iter().map(json_to_yml).collect()),
        Value::Object(values) => {
            let mut map = serde_yaml::Mapping::new();
            for (key, value) in values {
                map.insert(Yml::from(key.clone()), json_to_yml(value));
            }
            Yml::Mapping(map)
        }
    }
}
