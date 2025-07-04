use crate::{route_global, route_setting};
use axum::serve::Serve;
use axum::Router;
use library_core::core::{AnyResult, BizError, Exit};
use std::process::exit;
use std::sync::OnceLock;
use tokio::net::TcpListener;
use tokio::runtime::Runtime;

pub struct WebServerInner {
    pub port: u16,
    _runtime: Runtime,
}

impl WebServerInner {
    pub fn new(route: Router) -> AnyResult<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_io()
            .build()?;

        let port = runtime.block_on(_start(route))?;
        Ok(Self {
            port,
            _runtime: runtime,
        })
    }
}

async fn _start(route: Router) -> AnyResult<u16> {
    let target = "127.0.0.1:0";
    log::debug!("[Web] 绑定 {}", target);
    let bind = TcpListener::bind(target).await?;
    let addr = bind.local_addr()?;
    let port = addr.port();
    log::debug!("[Web] 获取当前绑定的端口 {}", port);
    let server = axum::serve(bind, route);
    tokio::spawn(async {
        log::debug!("[Web] 启动服务");
        match server.await {
            Ok(_) => {}
            Err(e) => {
                log::error!("服务异常退出: {:?}", e);
                exit(Exit::WebServerError.code())
            }
        }
    });
    Ok(port)
}

pub static SERVER: OnceLock<WebServerInner> = OnceLock::new();

pub fn start() -> AnyResult<()> {
    match SERVER.get() {
        None => {
            let mut router = Router::new();
            router = route_setting::fill(router);
            router = route_global::fill(router);

            let inner = WebServerInner::new(router)?;
            match SERVER.set(inner) {
                Ok(_) => Ok(()),
                Err(_) => Err(Box::new(BizError::WebUnset)),
            }
        }
        Some(_) => Ok(()),
    }
}
