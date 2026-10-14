//! cloudflare worker 入口。

mod convert;
mod core;
mod git;
mod params;
mod share;

use worker::Error::RustError;
use worker::*;

use crate::params::ConvertParams;

/// 内部统一使用的错误类型, 入口处转换为 [`worker::Error`]。
pub type AnyResult<T> = anyhow::Result<T>;
#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();
    let path = req.path().to_lowercase();
    console_debug!("path: {}", path);

    let response = if path == "/sing-box" {
        console_debug!("match: sing_box");
        Some(conversion(&req, &env, true).await)
    } else if path == "/clash" {
        console_debug!("match: clash");
        Some(conversion(&req, &env, false).await)
    } else if path.starts_with("/gist") {
        console_debug!("match: gist");
        git::gist(&req).await
    } else if path.starts_with("/release") {
        console_debug!("match: release");
        git::release(&req).await
    } else if path.starts_with("/latest") {
        console_debug!("match: release_latest");
        git::release_latest(&req).await
    } else if path.starts_with("/s/") {
        console_debug!("match: share");
        redirect(&req, &env).await
    } else {
        None
    };

    match response {
        Some(Ok(response)) => Ok(response),
        Some(Err(e)) => {
            console_error!("处理异常! {}", e);
            Err(RustError(e.to_string()))
        }
        None => Response::builder().with_status(403).ok("Deny access!"),
    }
}

/// 解析查询参数。
fn query(req: &Request) -> Result<std::collections::HashMap<String, Vec<String>>> {
    let url = req.url()?;
    let mut source: std::collections::HashMap<String, Vec<String>> = Default::default();

    for (key, value) in url.query_pairs() {
        source
            .entry(key.to_string())
            .or_default()
            .push(value.to_string());
    }

    Ok(source)
}

/// 解析转换参数。
fn params(req: &Request, env: &Env) -> Result<Option<ConvertParams>> {
    let source = query(req)?;
    Ok(ConvertParams::from_query(source, |key, p| {
        share::find_url(env, key, p)
    }))
}

/// 转换订阅并返回配置。
async fn conversion(req: &Request, env: &Env, sing_box: bool) -> AnyResult<Response> {
    let Some(params) = params(req, env)? else {
        return Ok(Response::builder().with_status(404).ok("not found")?);
    };

    let headers = core::forward_headers(req.headers());
    let (converted, ext) = if sing_box {
        (convert::sing_box(&params, headers.clone()).await, "json")
    } else {
        (convert::clash(&params, headers).await, "yaml")
    };
    let converted = converted?;

    let disposition = format!("inline; filename=\"{}.{}\"", converted.filename, ext);
    let mut builder = Response::builder()
        .with_header("Content-Disposition", &disposition)?
        .with_header("content-type", &format!("application/{}", ext))?;
    if let Some(info) = &converted.info {
        builder = builder.with_header("Subscription-Userinfo", info)?;
    }

    Ok(builder.fixed(converted.body.into_bytes()))
}

/// 分享源重定向。
async fn redirect(req: &Request, env: &Env) -> Option<AnyResult<Response>> {
    let path = req.path();
    let segments: Vec<&str> = path.split('/').collect();
    if segments.len() != 3 || segments[1] != "s" {
        return None;
    }

    let key = segments[2];
    let source = query(req).ok()?;
    let p = source.get("p")?.first()?;

    let url = share::find_url(env, key, p)?;
    let url = Url::parse(&url).ok()?;

    Some(Ok(Response::redirect_with_status(url, 307).ok()?))
}
