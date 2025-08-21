use crate::window::view_webview::WebView;
use library_core::core::AnyResult;
use tao::window::Window;

/// 视图
pub trait View {
    fn load(&self, url: &str) -> AnyResult<()>;
    fn eval(&self, js: &str) -> AnyResult<()>;
}
pub struct ViewWrapper {
    instance: Box<dyn View>,
}

impl ViewWrapper {
    pub fn wrap(instance: Box<dyn View>) -> Self {
        Self { instance }
    }
}

unsafe impl Send for ViewWrapper {}
unsafe impl Sync for ViewWrapper {}

impl View for ViewWrapper {
    fn load(&self, url: &str) -> AnyResult<()> {
        self.instance.load(url)
    }

    fn eval(&self, js: &str) -> AnyResult<()> {
        self.instance.eval(js)
    }
}

pub type OnPageLoad = dyn Fn() + 'static;

pub fn common_on_page_load<V: View>(v: &V) {
    let js_basic = r#"
        document.addEventListener('contextmenu', (e) => {
            e.preventDefault();
            return false;
        });
    "#;

    match v.eval(js_basic) {
        Ok(_) => {}
        Err(e) => {
            log::error!("通用页面加载js执行异常!")
        }
    }
}

pub fn with_html(
    window: &Window,
    html: &str,
    on_page_load: Box<OnPageLoad>,
) -> AnyResult<ViewWrapper> {
    let view = WebView::with_html(window, html, on_page_load)?;
    Ok(ViewWrapper::wrap(Box::new(view)))
}

pub fn with_url(
    window: &Window,
    url: &str,
    on_page_load: Box<OnPageLoad>,
) -> AnyResult<ViewWrapper> {
    let view = WebView::with_url(window, url, on_page_load)?;
    Ok(ViewWrapper::wrap(Box::new(view)))
}
