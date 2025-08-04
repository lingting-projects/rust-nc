#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::view::UiView;
use crate::window::{dispatch, WindowManager};
use library_core::core::AnyResult;
use std::thread;
use std::time::Duration;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop, EventLoopBuilder};
use tao::window::Window;
use tray_icon::TrayIconEvent;

mod icon;
mod init;
mod tray;
mod view;
mod window;

pub enum ExecuteEvent {
    Main(Box<dyn FnOnce(&Window, &dyn UiView) + Send>),
}

#[derive(Debug)]
pub enum UserEvent {
    EMPTY(),
    TrayIconEvent(TrayIconEvent),
    MenuEvent(tray_icon::menu::MenuEvent),
}

#[tokio::main]
async fn main() -> AnyResult<()> {
    library_core::app::init()?;
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let mut manager = WindowManager::new(&event_loop);
    let mut tray = Some(tray::create(&event_loop)?);

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(500));
        log::debug!("开始初始化");
        init::start_init();
        log::debug!("初始化完成");
    });

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event, window_id, ..
            } => {
                if window_id != manager.id() {
                    return;
                }
                manager.handler(&event);
                if event == WindowEvent::CloseRequested {
                    tray.take();
                    *control_flow = ControlFlow::Exit;
                }
            }

            Event::UserEvent(UserEvent::TrayIconEvent(e)) => {
                tray::handler_icon(&manager, e);
            }

            Event::UserEvent(UserEvent::MenuEvent(e)) => {
                tray::handler_menu(&manager, e);
            }
            Event::UserEvent(UserEvent::EMPTY()) => manager.receiver(),
            _ => return,
        }
    });
}
