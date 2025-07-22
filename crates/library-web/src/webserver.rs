use crate::route_subscribe::TIMER_SUBSCRIBE;
use crate::{route_global, route_setting, route_subscribe};
use axum::serve::Serve;
use axum::Router;
use library_core::core::{AnyResult, BizError, Exit};
use std::pin::Pin;
use std::process::exit;
use std::sync::{mpsc, Arc, LazyLock, Mutex, OnceLock};
use std::task::{Context, Poll};
use tokio::net::TcpListener;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;

#[derive(Debug)]
pub struct WebServer {
    pub port: u16,
    pub runtime: Option<Runtime>,
}

impl WebServer {}

pub static SERVER: OnceLock<Mutex<WebServer>> = OnceLock::new();

pub fn port() -> Option<u16> {
    SERVER.get().map(|m| m.lock().unwrap().port)
}

pub async fn build() -> AnyResult<(u16, Serve<TcpListener, Router, Router>)> {
    build_port(0).await
}

pub async fn build_port(port: u16) -> AnyResult<(u16, Serve<TcpListener, Router, Router>)> {
    let mut router = Router::new();
    router = route_setting::fill(router);
    router = route_subscribe::fill(router);

    // 这个必须最后设置
    router = route_global::fill(router);

    let target = format!("127.0.0.1:{}", port);
    log::debug!("[Web] 绑定 {}", target);
    let bind = TcpListener::bind(target).await?;
    let addr = bind.local_addr()?;
    let port = addr.port();
    log::debug!("[Web] 获取当前绑定的端口 {}", port);
    let server = axum::serve(bind, router);
    Ok((port, server))
}

static _TIMER_RUNTIME: OnceLock<Runtime> = OnceLock::new();

pub fn timer_wake() -> AnyResult<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .build()?;
    _TIMER_RUNTIME.get_or_init(move || {
        runtime.block_on(async {
            log::debug!("[Web] 唤醒定时器");
            TIMER_SUBSCRIBE.wake();
        });
        runtime
    });

    Ok(())
}
