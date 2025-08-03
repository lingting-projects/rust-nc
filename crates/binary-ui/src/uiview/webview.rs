use crate::uiview::UiView;
use library_core::core::AnyResult;
use tao::rwh_06::HasWindowHandle;
use wry::{WebView, WebViewBuilder};

pub struct UiWebView {
    inner: WebView,
}

impl UiWebView {
    pub fn new<W: HasWindowHandle>(
        window: &W,
        html: &str,
        with_page_load: fn(),
    ) -> AnyResult<UiWebView> {
        // 创建webview
        let builder = WebViewBuilder::new()
            .with_html(html)
            .with_on_page_load_handler(move |_, _| with_page_load());

        #[cfg(not(target_os = "linux"))]
        let webview = builder.build(&window).unwrap();

        #[cfg(target_os = "linux")]
        let webview = builder.build_gtk(window.gtk_window()).unwrap();

        Ok(Self { inner: webview })
    }
}

impl UiView for UiWebView {
    fn load(&self, url: &str) -> AnyResult<()> {
        self.inner.load_url(url)?;
        Ok(())
    }

    fn eval(&self, js: &str) -> AnyResult<()> {
        self.inner.evaluate_script(js)?;
        Ok(())
    }
}
