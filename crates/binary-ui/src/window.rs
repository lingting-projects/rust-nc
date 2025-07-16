use crate::init::FIRST;
use library_core::app::APP;
use library_core::core::{AnyResult, Exit};
use library_web::webserver;
use std::process::exit;
use std::sync::{
    mpsc::{channel, Receiver, Sender},
    OnceLock,
};
use tao::{
    dpi::PhysicalSize,
    event::Event,
    event_loop::{ControlFlow, EventLoop, EventLoopProxy},
    window::{Window, WindowBuilder},
};
use wry::{WebView, WebViewBuilder};

pub enum WindowEvent {
    ExecuteMain(Box<dyn FnOnce(&Window, &WebView) + Send>),
}

// 全局 sender，用于从其他模块发送事件
static SENDER: OnceLock<Sender<WindowEvent>> = OnceLock::new();
// 全局事件循环代理，用于唤醒事件循环
static LOOP_PROXY: OnceLock<EventLoopProxy<()>> = OnceLock::new();

// 对外提供的发送函数
pub fn send_event(event: WindowEvent) -> AnyResult<()> {
    if let Some(sender) = SENDER.get() {
        sender.send(event)?;
        // 唤醒事件循环处理新事件
        if let Some(proxy) = LOOP_PROXY.get() {
            proxy.send_event(())?
        }
    }
    Ok(())
}

// 新增 dispatch 函数，简化 ExecuteMain 事件的发送
pub fn dispatch<F>(closure: F) -> AnyResult<()>
where
    F: FnOnce(&Window, &WebView) + Send + 'static,
{
    send_event(WindowEvent::ExecuteMain(Box::new(closure)))
}

pub struct WindowManager {
    window: Window,
    webview: WebView,
    receiver: Receiver<WindowEvent>,
}

impl WindowManager {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        // 保存事件循环代理用于唤醒
        match LOOP_PROXY.set(event_loop.create_proxy()) {
            Ok(_) => {}
            Err(_) => {
                log::error!("事件代理设置异常!");
                exit(Exit::LoopProxyError.code())
            }
        };

        let app = APP.get().unwrap();
        // 创建窗口
        let size = PhysicalSize::new(1280, 960);
        let window = WindowBuilder::new()
            .with_title(FIRST.window_title())
            .with_visible(!app.run_on_minimize)
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(event_loop)
            .unwrap();

        let html = include_str!("../../../assets/loading.html");
        // 创建webview
        let builder = WebViewBuilder::new()
            .with_html(html)
            .with_on_page_load_handler(move |_, _| match webserver::port() {
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

                    dispatch(move |_, wv| wv.evaluate_script(&js).unwrap()).unwrap()
                }
            });

        #[cfg(not(target_os = "linux"))]
        let webview = builder.build(&window).unwrap();

        #[cfg(target_os = "linux")]
        let webview = builder.build_gtk(window.gtk_window()).unwrap();

        // 创建事件通道
        let (sender, receiver) = channel::<WindowEvent>();

        // 设置全局 sender
        match SENDER.set(sender) {
            Ok(_) => {}
            Err(_) => {
                log::error!("设置全局事件发送通道异常!");
                exit(Exit::WebViewSenderError.code())
            }
        }

        Self {
            window,
            webview,
            receiver,
        }
    }

    pub fn handle_event(&mut self, event: &Event<()>, control_flow: &mut ControlFlow) {
        match event {
            Event::WindowEvent {
                event: tao::event::WindowEvent::CloseRequested,
                window_id,
                ..
            } if *window_id == self.window.id() => {
                *control_flow = ControlFlow::Exit;
            }
            Event::NewEvents(_) => self.handle_recv(),
            Event::UserEvent(_) => self.handle_recv(),
            _ => {}
        }
    }

    fn handle_recv(&mut self) {
        // 处理自定义事件
        while let Ok(event) = self.receiver.try_recv() {
            match event {
                WindowEvent::ExecuteMain(closure) => {
                    closure(&self.window, &self.webview);
                }
            }
        }
    }
}
