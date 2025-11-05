mod convert;
mod core;
mod git;
mod share;

use worker::Error::RustError;
use worker::*;

#[event(fetch)]
async fn main(req: Request, env: Env, ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();
    let path = req.path().to_lowercase();
    console_debug!("path: {path}");
    let r = if path == "/sing-box" {
        console_debug!("match: sing_box");
        Some(convert::sing_box(req).await)
    } else if path == "/clash" {
        console_debug!("match: clash");
        Some(convert::clash(req).await)
    } else if path.starts_with("/gist") {
        console_debug!("match: gist");
        git::gist(req).await
    } else if path.starts_with("/release") {
        console_debug!("match: release");
        git::release(req).await
    } else if path.starts_with("/latest") {
        console_debug!("match: release_latest");
        git::release_latest(req).await
    } else if path.starts_with("/s/") {
        console_debug!("match: share");
        share::redirect(req, env).await
    } else {
        None
    };

    match r {
        Some(Ok(response)) => Ok(response),
        Some(Err(b)) => {
            let e = b.as_ref();
            console_error!("处理异常! {}", e);
            Err(RustError(e.to_string()))
        }
        None => Response::builder().with_status(403).ok("Deny access!"),
    }
}
