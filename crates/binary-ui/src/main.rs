#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::window::{dispatch, WindowManager};
use library_core::core::AnyResult;
use std::thread;
use std::time::Duration;
use tao::event::Event;
use tao::event_loop::{ControlFlow, EventLoop, EventLoopBuilder};

mod icon;
mod init;
mod tray;
mod view;
mod window;

#[derive(Debug)]
pub enum UserEvent {
    EMPTY(),
    TrayIconEvent(tray_icon::TrayIconEvent),
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
            Event::UserEvent(UserEvent::TrayIconEvent(_)) => {
                let _ = dispatch(|w, _| w.set_visible(true));
            }

            Event::UserEvent(UserEvent::MenuEvent(_)) => {
                // 比对事件id, 实现对应的事件
                tray.take();
            }
            e => manager.handle_event(&e, control_flow),
        }
    });
}
