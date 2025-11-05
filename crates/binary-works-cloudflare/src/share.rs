use crate::core::RequestExt;
use library_core::core::AnyResult;
use serde_json::{from_str, Map, Value};
use worker::{console_debug, console_error, Env, Request, Response, Url};

pub async fn redirect(req: Request, env: Env) -> Option<AnyResult<Response>> {
    let path = req.path();
    let segments: Vec<&str> = path.split('/').collect();

    if segments.len() != 3 || segments[1] != "s" {
        return None;
    }

    let key = segments[2];
    console_debug!("redirect: key={}", key);
    let share_env = env.var("share").ok()?;
    let share_json = share_env.as_ref().as_string()?;
    console_debug!("redirect: share_json={}", share_json);
    let share_value = from_str::<Value>(&share_json).ok()?;
    let share = match share_value {
        Value::Object(o) => o,
        _ => {
            console_error!("json parser result error! json: {}", &share_json);
            return None;
        }
    };
    console_debug!("redirect: share size: {}", share.len());
    let v = share.get(key)?;
    match v {
        Value::String(s) => redirect_str(s),
        Value::Object(map) => redirect_obj(req, map),
        _ => None,
    }
}

fn redirect_str(v: &str) -> Option<AnyResult<Response>> {
    let url = v.parse::<Url>().ok()?;
    let response = Response::redirect_with_status(url, 307).ok()?;
    Some(Ok(response))
}

fn redirect_obj(req: Request, v: &Map<String, Value>) -> Option<AnyResult<Response>> {
    let password = v.get("p").unwrap();
    let map = req.query_map().ok()?;
    let input_password = map.get("p")?.first()?;

    match password {
        Value::String(p) => {
            if input_password == p {
                let x = v.get("url")?;
                if let Value::String(u) = x {
                    return redirect_str(u);
                }
            }
        }
        _ => {}
    };
    None
}
