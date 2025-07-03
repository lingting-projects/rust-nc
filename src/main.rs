mod init;
mod logging;
mod webview;

use std::process::exit;
use std::thread;
use std::time::Duration;
use thiserror::__private::AsDynError;

fn main() {
    logging::init();
    let webview = webview::create();

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(300));
        log::debug!("开始初始化");
        match init::start_init() {
            Ok(_) => {
                log::debug!("初始化完成")
            }
            Err(e) => {
                log::error!("初始化异常! {}", e.as_dyn_error());
                webview::exit()
            }
        }
    });

    // 运行WebView事件循环
    match webview.run() {
        Ok(_) => {}
        Err(e) => {
            log::error!("WebView error: {}", e);
            exit(1)
        }
    }
}
