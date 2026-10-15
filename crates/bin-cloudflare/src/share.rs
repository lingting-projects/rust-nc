use serde_json::{Value, from_str};
use worker::{Env, console_debug, console_error};

/// 环境变量名称。
const VARIABLE: &str = "share";

/// 由环境变量解析分享源地址, `key` 为分享键, `p` 为分享参数。
pub fn find_url(env: &Env, key: &str, p: &str) -> Option<String> {
    console_debug!("share: key={}, p={}", key, p);
    let source = env.var(VARIABLE).ok()?.to_string();
    console_debug!("share: {}", source);

    let root = match from_str::<Value>(&source) {
        Ok(root) => root,
        Err(e) => {
            console_error!("share 解析失败! {}", e);
            return None;
        }
    };

    url(root.get(key)?, p)
}

/// 由分享值解析地址, 值为对象时要求 `p` 与配置一致。
fn url(value: &Value, p: &str) -> Option<String> {
    match value {
        Value::String(url) => Some(url.clone()),
        Value::Object(map) => {
            if p.is_empty() || map.get("p")?.as_str()? != p {
                return None;
            }
            map.get("u")?.as_str().map(|url| url.to_string())
        }
        _ => None,
    }
}
