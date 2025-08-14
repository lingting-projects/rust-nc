use crate::init::FIRST;
use crate::view::UiView;
use crate::{view, ExecuteEvent, UserEvent};
use library_core::core::{AnyResult, Exit};
use library_web::webserver;
use std::fmt::format;
use std::process::exit;
use std::sync::{
    mpsc::{channel, Receiver, Sender},
    OnceLock,
};
use tao::event::WindowEvent;
use tao::event_loop::EventLoopClosed;
use tao::window::WindowId;
use tao::{
    dpi::PhysicalSize,
    event::Event,
    event_loop::{ControlFlow, EventLoop, EventLoopProxy},
    window::{Window, WindowBuilder},
};
use wry::{WebView, WebViewBuilder};

// 全局 sender，用于从其他模块发送事件
static SENDER: OnceLock<Sender<ExecuteEvent>> = OnceLock::new();
// 全局事件循环代理，用于唤醒事件循环
static LOOP_PROXY: OnceLock<EventLoopProxy<UserEvent>> = OnceLock::new();

pub trait WindowExt {
    fn force_show(&self);
}

impl WindowExt for Window {
    fn force_show(&self) {
        self.set_visible(true);
        self.set_minimized(false);
        self.set_focus();
    }
}

// 发送函数
fn send_event(event: ExecuteEvent) -> AnyResult<()> {
    if let Some(sender) = SENDER.get() {
        sender
            .send(event)
            .map_err(|e| format!("事件发生异常! {e}"))?;
        // 唤醒事件循环处理新事件
        if let Some(proxy) = LOOP_PROXY.get() {
            proxy.send_event(UserEvent::EMPTY())?
        }
    }
    Ok(())
}

// 新增 dispatch 函数，简化事件的发送
pub fn dispatch<F>(closure: F) -> AnyResult<()>
where
    F: FnOnce(&Window, &dyn UiView) + Send + 'static,
{
    send_event(ExecuteEvent::Main(Box::new(closure)))
}

pub struct WindowManager {
    window: Window,
    ui: Box<dyn UiView>,
    receiver: Receiver<ExecuteEvent>,
}

impl WindowManager {
    pub fn id(&self) -> WindowId {
        self.window.id()
    }

    pub fn show(&self) {
        self.window.force_show()
    }

    pub fn hidden(&self) {
        self.window.set_visible(false);
    }

    pub fn new(event_loop: &EventLoop<UserEvent>) -> Self {
        // 保存事件循环代理用于唤醒
        match LOOP_PROXY.set(event_loop.create_proxy()) {
            Ok(_) => {}
            Err(_) => {
                log::error!("事件代理设置异常!");
                exit(Exit::LoopProxyError.code())
            }
        };

        // 创建事件通道
        let (sender, receiver) = channel::<ExecuteEvent>();

        // 设置全局 sender
        match SENDER.set(sender) {
            Ok(_) => {}
            Err(_) => {
                log::error!("设置全局事件发送通道异常!");
                exit(Exit::WebViewSenderError.code())
            }
        }

        // 创建窗口
        let size = PhysicalSize::new(1280, 960);
        let window = WindowBuilder::new()
            .with_title(FIRST.window_title())
            .with_visible(false)
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(event_loop)
            .unwrap();

        let ui = view::new(&window).expect("ui view init err");

        Self {
            window,
            ui,
            receiver,
        }
    }

    pub fn handler(&self, event: &WindowEvent) {
        match event {
            WindowEvent::Focused(focused) => {
                if !focused && self.window.is_minimized() {
                    self.hidden();
                }
            }
            _ => {}
        }
    }

    pub fn receiver(&mut self) {
        // 处理自定义事件
        while let Ok(event) = self.receiver.try_recv() {
            match event {
                ExecuteEvent::Main(closure) => {
                    closure(&self.window, self.ui.as_ref());
                }
            }
        }
    }
}
