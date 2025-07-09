mod convert;
mod core;

use library_nc::core::AnyResult;
use worker::Error::RustError;
use worker::*;

async fn default(req: Request, env: Env, ctx: Context) -> AnyResult<Response> {
    Ok(Response::builder().with_status(403).ok("Deny access!")?)
}

#[event(fetch)]
async fn main(req: Request, env: Env, ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();
    let r = match req.path().to_lowercase().as_str() {
        "/sing-box" => convert::sing_box(req).await,
        "/clash" => convert::clash(req).await,
        _ => default(req, env, ctx).await,
    };
    match r {
        Ok(response) => Ok(response),
        Err(b) => {
            let e = b.as_ref();
            console_error!("处理异常! {}", e);
            Err(RustError(e.to_string()))
        }
    }
}
