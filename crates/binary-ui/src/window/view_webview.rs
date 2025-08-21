use crate::window::view::{OnPageLoad, View};
use library_core::app::get_app;
use library_core::core::AnyResult;
use library_core::file;
use tao::window::Window;
use wry::WebViewBuilder;

pub struct WebView {
    inner: wry::WebView,
}

impl View for WebView {
    fn load(&self, url: &str) -> AnyResult<()> {
        self.inner.load_url(url)?;
        Ok(())
    }

    fn eval(&self, js: &str) -> AnyResult<()> {
        self.inner.evaluate_script(js)?;
        Ok(())
    }
}

impl WebView {
    pub fn with_html(
        window: &Window,
        html: &str,
        on_page_load: Box<OnPageLoad>,
    ) -> AnyResult<Self> {
        let builder = builder(on_page_load)?.with_html(html);

        let view = build(window, builder)?;
        Ok(Self { inner: view })
    }
    pub fn with_url(
        window: &Window,
        url: &str,
        on_page_load: Box<OnPageLoad>,
    ) -> AnyResult<Self> {
        let builder = builder(on_page_load)?.with_url(url);

        let view = build(window, builder)?;
        Ok(Self { inner: view })
    }
}

fn builder(on_page_load: Box<OnPageLoad>) -> AnyResult<WebViewBuilder<'static>> {
    let mut devtools = false;
    if cfg!(not(feature = "prod")) {
        devtools = true;
    }

    let app = get_app();
    let dir = app.cache_dir.join("webview");
    file::create_dir(&dir)?;
    log::debug!("用户数据: {}", dir.display());
    // todo 等wry实现了指定用户数据目录

    let builder = WebViewBuilder::new()
        .with_autoplay(false)
        .with_devtools(devtools)
        .with_on_page_load_handler(move |_, _| on_page_load());
    Ok(builder)
}

fn build(window: &Window, builder: WebViewBuilder) -> AnyResult<wry::WebView> {
    #[cfg(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "ios",
        target_os = "android"
    ))]
    let webview = builder.build(window)?;
    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "ios",
        target_os = "android"
    )))]
    let webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        let vbox = window.default_vbox()?;
        builder.build_gtk(vbox)?
    };

    Ok(webview)
}
