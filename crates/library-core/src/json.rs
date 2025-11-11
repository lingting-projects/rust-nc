use serde_json::Value;

pub trait JsonValueExt {
    fn string(&self) -> Option<String>;

    #[cfg(feature = "yml")]
    fn yml(&self) -> serde_yaml::Value;
}

impl JsonValueExt for Value {
    fn string(&self) -> Option<String> {
        match self {
            Value::Null => None,
            Value::Bool(v) => Some(v.to_string()),
            Value::Number(v) => Some(v.to_string()),
            Value::String(v) => Some(v.clone()),
            Value::Array(v) => serde_json::to_string(&v).ok(),
            Value::Object(v) => serde_json::to_string(&v).ok(),
        }
    }

    #[cfg(feature = "yml")]
    fn yml(&self) -> serde_yaml::Value {
        match self {
            Value::Null => serde_yaml::Value::Null,
            Value::Bool(v) => serde_yaml::Value::from(v.clone()),
            Value::Number(v) => {
                if v.is_f64() {
                    if let Some(n) = v.as_f64() {
                        return serde_yaml::Value::from(n);
                    }
                } else if v.is_i64() {
                    if let Some(n) = v.as_i64() {
                        return serde_yaml::Value::from(n);
                    }
                } else {
                    if let Some(n) = v.as_u64() {
                        return serde_yaml::Value::from(n);
                    }
                }
                serde_yaml::Value::Null
            }
            Value::String(v) => serde_yaml::Value::from(v.clone()),
            Value::Array(v) => {
                let vec: Vec<serde_yaml::Value> = v.into_iter().map(|_v| _v.yml()).collect();
                serde_yaml::Value::from(vec)
            }
            Value::Object(v) => {
                let mut map = serde_yaml::value::Mapping::new();
                for (_k, _v) in v {
                    let k = serde_yaml::Value::from(_k.clone());
                    let v = _v.yml();
                    map.insert(k, v);
                }
                serde_yaml::Value::from(map)
            }
        }
    }
}
