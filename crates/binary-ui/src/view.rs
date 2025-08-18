use crate::view::webview::UiWebView;
use crate::window::dispatch;
use library_core::core::AnyResult;
use library_web::webserver;
use tao::rwh_06::HasWindowHandle;

mod webview;

pub trait UiView {
    fn load(&self, url: &str) -> AnyResult<()>;
    fn eval(&self, js: &str) -> AnyResult<()>;
}

fn with_page_load(view: &dyn UiView) {
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
            view.eval(&js).expect("failed set request prefix");
        }
    }

    let js_basic = r#"
        document.addEventListener('contextmenu', (e) => {
            e.preventDefault();
            return false;
        });
    "#;

    view.eval(js_basic).expect("failed eval basic js");
}

pub fn new<W: HasWindowHandle>(window: &W) -> AnyResult<Box<dyn UiView>> {
    let html = include_str!("../../../assets/loading.html");
    let view = UiWebView::new(window, html, || {
        dispatch(|_, wv| with_page_load(wv)).expect("failed dispatch with page load")
    })?;
    Ok(Box::new(view))
}
