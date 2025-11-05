use crate::core::RequestExt;
use library_core::core::AnyResult;
use serde_json::{from_str, Value};
use worker::{console_debug, console_error, Env, Request, Response, Url};

pub fn find_url(env: Env, key: &str, p: Option<&String>) -> Option<String> {
    console_debug!("share: key={}", key);
    if let Some(_p) = p {
        console_debug!("share: p={}", _p);
    }

    let share_env = env.var("share").ok()?;
    let share_json = share_env.as_ref().as_string()?;
    console_debug!("share: share_json={}", share_json);
    let share_value = from_str::<Value>(&share_json).ok()?;
    let share = match share_value {
        Value::Object(o) => o,
        _ => {
            console_error!("json parser result error! json: {}", &share_json);
            return None;
        }
    };
    console_debug!("share: size: {}", share.len());
    let v = share.get(key)?;
    get_url(v, p)
}

fn get_url(v: &Value, io: Option<&String>) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Object(o) => {
            let ip = io?;
            if ip.is_empty() {
                return None;
            }
            if let Value::String(p) = o.get("p")?
                && p == ip
            {
                if let Value::String(s) = o.get("u")? {
                    return Some(s.clone());
                }
            }
            None
        }
        _ => None,
    }
}

pub async fn redirect(req: Request, env: Env) -> Option<AnyResult<Response>> {
    let path = req.path();
    let segments: Vec<&str> = path.split('/').collect();

    if segments.len() != 3 || segments[1] != "s" {
        return None;
    }

    let key = segments[2];
    let map = req.query_map().ok()?;
    let p = map.get("p")?.first();

    let u = find_url(env, key, p)?;
    let url = Url::parse(&u).ok()?;
    let response = Response::redirect_with_status(url, 307).ok()?;
    Some(Ok(response))
}
