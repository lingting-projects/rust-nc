#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::single::Single;
use crate::view::UiView;
use crate::window::{dispatch, WindowManager};
use library_core::app::APP;
use library_core::core::{AnyResult, BizError};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop, EventLoopBuilder};
use tao::window::Window;
use tray_icon::TrayIconEvent;

mod icon;
mod init;
mod single;
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

fn create_single(path: PathBuf, info: &str) -> AnyResult<Option<Single>> {
    let single = Single::create(path, info)?;
    if !single.is_single {
        log::error!("存在已启动进程: {}", single.pid.unwrap_or(0));
        log::error!("已启动进程info: {}", single.info);
        Err(Box::new(BizError::NoSingle))
    } else {
        Ok(Some(single))
    }
}

#[tokio::main]
async fn main() -> AnyResult<()> {
    library_core::app::init()?;
    let app = APP.get().expect("get app failed");
    let lock_path = app.cache_dir.join("single.lock");
    let mut o_single = create_single(lock_path, "ipc info")?;
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
                    o_single.take();
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
