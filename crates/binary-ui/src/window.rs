mod main;
pub mod view;
mod view_webview;

use crate::init::FIRST;
use crate::window::view::{OnPageLoad, View, ViewWrapper};
use crate::{icon, tray, UserEvent};
use library_core::core::{AnyResult, BizError, Exit};
use std::collections::HashMap;
use std::process::exit;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, LazyLock, OnceLock};
use tao::dpi::PhysicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{
    ControlFlow, EventLoop, EventLoopClosed, EventLoopProxy, EventLoopWindowTarget,
};
use tao::window::{WindowBuilder, WindowId};
use tray_icon::{TrayIcon, TrayIconEvent};

type Callback = dyn FnOnce(&tao::window::Window, &ViewWrapper) + Send;

pub enum NcChannelEvent {
    Main(Box<Callback>),
    Window(WindowId, Box<Callback>),
}

static p: OnceLock<Arc<EventLoopProxy<UserEvent>>> = OnceLock::new();
static s: OnceLock<Sender<NcChannelEvent>> = OnceLock::new();

fn _dispatch(e: NcChannelEvent) -> AnyResult<()> {
    if let Some(sender) = s.get() {
        match sender.send(e) {
            Ok(_) => {
                if let Some(proxy) = p.get() {
                    match proxy.send_event(UserEvent::EMPTY()) {
                        Ok(_) => Ok(()),
                        Err(e) => {
                            log::error!("主进程唤醒事件推送失败! {}", e);
                            Err(Box::new(BizError::EventSend("主进程唤醒".to_string())))
                        }
                    }
                } else {
                    Err(Box::new(BizError::Unsupported))
                }
            }
            Err(e) => {
                log::error!("主进程回调事件推送失败! {}", e);
                Err(Box::new(BizError::EventSend("主进程回调".to_string())))
            }
        }
    } else {
        Err(Box::new(BizError::Unsupported))
    }
}

pub fn dispatch(c: Box<Callback>) -> AnyResult<()> {
    _dispatch(NcChannelEvent::Main(c))
}

pub fn dispatch_window(id: WindowId, c: Box<Callback>) -> AnyResult<()> {
    _dispatch(NcChannelEvent::Window(id, c))
}

pub enum NcWindowEvent {
    OpenKernel(String),
}

pub struct Window {
    tray: Option<TrayIcon>,
    map: HashMap<WindowId, (tao::window::Window, ViewWrapper)>,
    pub main: WindowId,
    r: Receiver<NcChannelEvent>,
}

impl Window {
    pub fn new(l: &EventLoop<UserEvent>) -> AnyResult<Self> {
        let p_arc = Arc::new(l.create_proxy());
        match p.set(p_arc) {
            Ok(_) => {}
            Err(_) => {
                log::error!("事件代理设置异常!");
                exit(Exit::LoopProxyError.code())
            }
        };

        let (_s, r) = mpsc::channel::<NcChannelEvent>();
        match s.set(_s) {
            Ok(_) => {}
            Err(_) => {
                log::error!("设置全局事件发送通道异常!");
                exit(Exit::WebViewSenderError.code())
            }
        }
        let size = PhysicalSize::new(1440, 1082);
        let window = main::build_window(l, builder_window(size, FIRST.window_title()))?;
        let id = window.id();
        let view = main::build_view(&window)?;
        let tray = tray::create(l)?;

        let mut map = HashMap::new();
        map.insert(id.clone(), (window, view));

        Ok(Self {
            tray: Some(tray),
            map,
            main: id,
            r,
        })
    }

    pub fn create<F: Fn(WindowId) + 'static>(
        &mut self,
        l: &EventLoopWindowTarget<UserEvent>,
        title: &str,
        url: String,
        on_page_load: F,
    ) -> AnyResult<()> {
        let size = PhysicalSize::new(1280, 960);
        let window = builder_window(size, title).build(l)?;
        let id = window.id();
        let view = view::with_url(&window, &url, Box::new(move || on_page_load(id)))?;
        self.map.insert(window.id(), (window, view));
        Ok(())
    }

    pub fn on_event(
        &mut self,
        event: Event<UserEvent>,
        target: &EventLoopWindowTarget<UserEvent>,
    ) -> ControlFlow {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
                ..
            } => {
                // 隐藏要移除的窗口
                self.consumer(window_id, |w, _| w.set_visible(false));
                // 移除子窗口
                if window_id != self.main {
                    self.map.remove(&window_id);
                }
                // 移除主窗口
                else {
                    self.map.clear();
                    self.tray.take();
                    return ControlFlow::Exit;
                }
            }
            Event::WindowEvent {
                event: WindowEvent::Focused(focused),
                window_id,
                ..
            } => {
                if window_id == self.main {
                    self.consumer(window_id, |w, _| {
                        // 未选中主窗口且主窗口已经最小化
                        if !focused && w.is_minimized() {
                            w.set_visible(false)
                        }
                    })
                }
            }

            Event::UserEvent(UserEvent::TrayIconEvent(e)) => {
                tray::handler_icon(self, e);
            }

            Event::UserEvent(UserEvent::MenuEvent(e)) => {
                tray::handler_menu(self, e);
            }

            Event::UserEvent(UserEvent::NcWindowEvent(NcWindowEvent::OpenKernel(url))) => {
                match self.create(
                    target,
                    "内核管理",
                    url.clone(),
                    Box::new(|id| {
                        match dispatch_window(
                            id,
                            Box::new(|w, v| match on_kernel_page_load(w, v) {
                                Ok(_) => {}
                                Err(e) => {
                                    log::error!("内核管理界面加载完成回调执行异常! {}", e)
                                }
                            }),
                        ) {
                            Ok(_) => {}
                            Err(e) => {
                                log::error!("内核管理界面加载完成回调分发异常! {}", e)
                            }
                        }
                    }),
                ) {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("创建内核界面异常! url: {}; {}", url, e)
                    }
                }
            }

            Event::UserEvent(UserEvent::EMPTY()) => {
                if let Ok(c) = self.r.try_recv() {
                    let (id, c) = match c {
                        NcChannelEvent::Main(c) => (self.main, c),
                        NcChannelEvent::Window(id, c) => (id, c),
                    };

                    self.consumer(id, |w, v| c(w, v))
                }
            }

            _ => {}
        }

        ControlFlow::Wait
    }
}

impl Window {
    pub fn consumer<F: FnOnce(&tao::window::Window, &ViewWrapper)>(&self, id: WindowId, f: F) {
        self.map.get(&id).map(|(w, v)| f(w, v));
    }

    pub fn show(&self, id: WindowId) {
        self.consumer(id, |w, _| w.focus_show());
    }
}

fn builder_window(size: PhysicalSize<i32>, title: &str) -> WindowBuilder {
    WindowBuilder::new()
        .with_title(title)
        .with_visible(false)
        .with_inner_size(size)
        .with_min_inner_size(size)
}

fn on_kernel_page_load(w: &tao::window::Window, v: &ViewWrapper) -> AnyResult<()> {
    w.set_window_icon(Some(icon::tao()?));

    let size = PhysicalSize::new(1620, 810);
    w.set_inner_size(size);
    w.set_min_inner_size(Some(size));
    w.focus_show();
    let js = format!(
        // 覆盖节点设置, 主题仅在不存在时设置
        r#"
        localStorage.setItem('endpointList', '[{{"id":"55f9cc9d-3523-414a-bbe1-f9ec747fbf1e","url":"http://127.0.0.1:9090","secret":""}}]');
        localStorage.setItem('selectedEndpoint','"55f9cc9d-3523-414a-bbe1-f9ec747fbf1e"');
        !localStorage.theme&&localStorage.setItem('theme','"corporate"');
    "#
    );
    v.eval(&js)?;
    Ok(())
}

pub trait TaoWindowExt {
    fn focus_show(&self);
}

impl TaoWindowExt for tao::window::Window {
    fn focus_show(&self) {
        self.set_visible(true);
        self.set_minimized(false);
        self.set_focus();
    }
}
