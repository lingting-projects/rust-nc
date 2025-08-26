use serde_yaml::Value;

pub trait YmlValueExt {
    fn string(&self) -> Option<String>;
    fn string_empty(&self) -> String;
    #[cfg(feature = "json")]
    fn json(&self) -> serde_json::Value;
}

impl YmlValueExt for Value {
    fn string(&self) -> Option<String> {
        let s = match self {
            Value::Bool(v) => v.to_string(),
            Value::Number(v) => v.to_string(),
            Value::String(v) => v.clone(),
            _ => return None,
        };
        Some(s)
    }

    fn string_empty(&self) -> String {
        self.string().unwrap_or_else(|| String::new())
    }

    #[cfg(feature = "json")]
    fn json(&self) -> serde_json::Value {
        match self {
            Value::Bool(v) => serde_json::Value::Bool(v.clone()),
            Value::Number(v) => {
                if v.is_nan() {
                    serde_json::Value::from(f64::NAN)
                } else if v.is_infinite() {
                    serde_json::Value::from(f64::INFINITY)
                } else if v.is_f64() {
                    serde_json::Value::from(v.as_f64())
                } else if v.is_i64() {
                    serde_json::Value::from(v.as_i64())
                } else {
                    serde_json::Value::from(v.as_u64())
                }
            }
            Value::String(v) => serde_json::Value::String(v.clone()),
            Value::Sequence(v) => {
                let vec: Vec<serde_json::Value> =
                    v.to_vec().into_iter().map(|_v| _v.json()).collect();
                serde_json::Value::Array(vec)
            }
            Value::Mapping(v) => {
                let mut map = serde_json::value::Map::new();
                for (_k, _v) in v {
                    let k = _k.string_empty();
                    let v = _v.json();
                    map.insert(k, v);
                }

                serde_json::Value::Object(map)
            }
            _ => serde_json::Value::Null,
        }
    }
}
