use crate::view::UiView;
use crate::window::{dispatch, WindowExt, WindowManager};
use library_core::app::get_app;
use library_core::core::{panic_msg, AnyResult, BizError};
use library_core::file;
use rust_single::{Single, SingleBuild};
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{panic, thread};
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

fn release_single(single: Single) {
    let path = single.path.clone();
    let path_info = single.path_info.clone();
    drop(single);
    let _ = file::delete(&path);
    let _ = file::delete(&path_info);
}

fn create_single(path: PathBuf) -> AnyResult<Option<Single>> {
    let single = SingleBuild::new(path)
        .with_ipc(|_| {
            let _ = dispatch(|w, _| w.force_show());
        })
        .build()?;
    if !single.is_single {
        log::error!("存在已启动进程: {}", single.pid.unwrap_or(0));
        log::error!("已启动进程Ipc: {}", single.info);
        if let Err(e) = single.wake("") {
            log::error!("已启动进程唤醒异常! {}", e)
        }
        Err(Box::new(BizError::SingleRunning))
    } else {
        Ok(Some(single))
    }
}

#[tokio::main]
async fn main() -> AnyResult<()> {
    #[cfg(all(target_os = "windows", not(debug_assertions)))]
    {
        use std::ptr;
        use winapi::um::wincon::GetConsoleWindow;
        use winapi::um::winuser::{ShowWindow, SW_HIDE};
        let console_window = unsafe { GetConsoleWindow() };

        if console_window != ptr::null_mut() {
            // 隐藏窗口
            unsafe {
                ShowWindow(console_window, SW_HIDE);
            }
        }
    }

    library_core::app::init()?;
    let app = get_app();
    let lock_path = app.cache_dir.join("single.lock");
    let mut o_single = create_single(lock_path)?;
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
