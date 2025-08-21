use crate::window::view::{View, ViewWrapper};
use crate::window::view_webview::WebView;
use crate::window::{dispatch, view, NcWindowEvent};
use crate::UserEvent;
use library_core::core::AnyResult;
use library_web::webserver;
use std::sync::Arc;
use tao::event_loop::{EventLoop, EventLoopClosed, EventLoopProxy};
use tao::window::{Window, WindowBuilder};

pub fn build_window(l: &EventLoop<UserEvent>, builder: WindowBuilder) -> AnyResult<Window> {
    let window = builder.build(l)?;
    Ok(window)
}

fn on_page_load() {
    let result = dispatch(Box::new(|w, v| {
        match webserver::port() {
            None => {}
            Some(port) => {
                let api = format!("http://localhost:{}", port);
                let js = format!(
                    r#"
        try {{
            localStorage.setItem("nc:requestPrefix", "{}");
            window.requestPrefix="{}";
            window.setRequestPrefix && window.setRequestPrefix("{}");
        }} catch(e) {{
            console.error("初始化请求前缀异常!", e);
        }}
        "#,
                    api, api, api
                );
                v.eval(&js).expect("failed set request prefix");
            }
        }

        let js_basic = r#"
        document.addEventListener('contextmenu', (e) => {
            e.preventDefault();
            return false;
        });
    "#;

        v.eval(js_basic).expect("failed eval basic js");
    }));
    match result {
        Ok(_) => {}
        Err(e) => {
            log::error!("主窗口视图加载时回调异常! {}", e)
        }
    }
}

pub fn build_view(
    window: &Window,
) -> AnyResult<ViewWrapper> {
    let view = view::with_html(
        window,
        include_str!("../../../../assets/loading.html"),
        Box::new(move || on_page_load()),
    )?;
    Ok(view)
}
