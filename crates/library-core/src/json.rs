use serde_json::Value;

pub trait JsonValueExt {
    fn string(self) -> Option<String>;
}

impl JsonValueExt for Value {
    fn string(self) -> Option<String> {
        match self {
            Value::Null => None,
            Value::Bool(v) => Some(v.to_string()),
            Value::Number(v) => Some(v.to_string()),
            Value::String(v) => Some(v),
            Value::Array(v) => serde_json::to_string(&v).ok(),
            Value::Object(v) => serde_json::to_string(&v).ok(),
        }
    }
}
