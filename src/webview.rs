use crate::init::FIRST;
use once_cell::sync::Lazy;
use std::error::Error;
use std::sync::{Mutex, MutexGuard, PoisonError};
use web_view::{Content, Handle};

#[derive(thiserror::Error, Debug)]
pub enum WebViewError {
    #[error("WebView未初始化")]
    NON,
    #[error("WebView锁异常 {0}")]
    LOCK(Box<dyn Error>),
}

pub static WEBVIEW: Lazy<Mutex<Option<Handle<()>>>> = Lazy::new(|| Mutex::new(None));

pub fn handle() -> Result<Handle<()>, WebViewError> {
    match WEBVIEW.lock() {
        Ok(guard) => {
            let handle = guard.clone().ok_or(WebViewError::NON)?;
            Ok(handle.clone())
        }
        Err(e) => Err(WebViewError::LOCK(Box::new(e))),
    }
}

pub fn exit() {
    match handle() {
        Ok(handle) => {
            let _ = handle.dispatch(move |wv| {
                wv.exit();
                Ok(())
            });
        }
        Err(_) => {}
    }
}

pub fn create() -> web_view::WebView<'static, ()> {
    log::info!("WebView初始化开始");
    // 创建WebView，使用带前缀的标题
    let html = include_str!("../resources/loading.html");
    let content = Content::Html(html);
    let webview = web_view::builder()
        .title(FIRST.window_title())
        .content(content)
        .size(1024, 768)
        .resizable(false)
        .visible(false)
        .debug(true)
        .user_data(())
        .invoke_handler(|_wv, arg| {
            // 处理JavaScript调用
            log::debug!("Received message from JS: {}", arg);
            Ok(())
        })
        .build()
        .unwrap();

    log::info!("WebView初始化完成");
    // 将 WebViewHandle 存储到全局静态变量中
    let mut handle_guard = WEBVIEW.lock().unwrap();
    *handle_guard = Some(webview.handle());
    webview
}
