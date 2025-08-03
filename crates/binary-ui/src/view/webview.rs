use crate::view::UiView;
use library_core::app::APP;
use library_core::core::AnyResult;
use library_core::file;
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
            .with_autoplay(false)
            .with_on_page_load_handler(move |_, _| with_page_load());

        #[cfg(windows)]
        {
            let app = APP.get().expect("failed get app context");
            let dir = app.cache_dir.join("webview");
            file::create_dir(&dir)?;
            let dir_str = dir.to_str().expect("failed get dir path");
            log::debug!("用户数据: {}", dir_str)
            // todo 等wry实现了指定用户数据目录
        }

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
