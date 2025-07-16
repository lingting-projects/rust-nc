use library_core::core::{AnyResult, Exit};
use library_web::webserver;
use library_web::webserver::{WebServer, SERVER};
use log::log;
use std::process::exit;
use std::sync::Mutex;

#[tokio::main]
async fn main() -> AnyResult<()> {
    library_core::app::init();
    log::debug!("构建服务!");
    let (port, server) = webserver::build().await?;
    log::debug!("绑定到端口: {}", port);

    let web_server = WebServer {
        port,
        runtime: None,
    };
    SERVER
        .set(Mutex::new(web_server))
        .expect("unset web server");
    log::debug!("启动服务!");
    match server.await {
        Ok(_) => {}
        Err(e) => {
            log::error!("服务启动异常! {}", e);
            exit(Exit::WebServerError.code())
        }
    };

    log::debug!("服务已关闭!");
    Ok(())
}
