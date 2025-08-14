#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::ipc::{IpcServer, IpcStream};
use crate::single::Single;
use crate::view::UiView;
use crate::window::{dispatch, WindowExt, WindowManager};
use library_core::app::get_app;
use library_core::core::{panic_msg, AnyResult, BizError};
use library_core::file;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{panic, thread};
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop, EventLoopBuilder};
use tao::window::Window;
use tray_icon::TrayIconEvent;

mod icon;
mod init;
mod ipc;
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

fn wake(ipc_path: &str) -> AnyResult<()> {
    let mut stream = IpcStream::new(ipc_path)?;
    stream.write("")
}

fn release_single(single: Single) {
    let path = single.path.clone();
    let path_info = single.path_info.clone();
    drop(single);
    let _ = file::delete(&path);
    let _ = file::delete(&path_info);
}

fn create_single(path: PathBuf, ipc_path: &str) -> AnyResult<Option<Single>> {
    let single = Single::create(path, ipc_path)?;
    if !single.is_single {
        log::error!("存在已启动进程: {}", single.pid.unwrap_or(0));
        log::error!("已启动进程Ipc: {}", single.info);
        if let Err(e) = wake(&single.info) {
            log::error!("已启动进程唤醒异常! {}", e)
        }
        Err(Box::new(BizError::SingleRunning))
    } else {
        log::debug!("创建ipc服务: {}", ipc_path);
        match IpcServer::new(ipc_path) {
            Ok(server) => {
                thread::spawn(move || {
                    loop {
                        match panic::catch_unwind(|| server.next()) {
                            Ok(Ok(_)) => {
                                let _ = dispatch(|w, _| w.force_show());
                            }
                            Ok(Err(e)) => {
                                log::error!("ipc server read err! {}", e)
                            }
                            Err(p) => {
                                log::error!("ipc server read err! {}", panic_msg(p))
                            }
                        }
                    }
                });
            }
            Err(e) => {
                release_single(single);
                log::error!("创建ipc服务异常! {}", e);
                return Err(e);
            }
        }
        Ok(Some(single))
    }
}

#[tokio::main]
async fn main() -> AnyResult<()> {
    library_core::app::init()?;
    let app = get_app();
    let lock_path = app.cache_dir.join("single.lock");
    let _ipc_path = app.cache_dir.join("ipc.socket");
    let ipc_path = _ipc_path.to_str().expect("failed get ipc path");
    let mut o_single = create_single(lock_path, ipc_path)?;
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
                    let o = o_single.take();
                    if let Some(single) = o {
                        release_single(single)
                    }
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
