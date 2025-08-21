use crate::init::FIRST;
use crate::window::{dispatch, NcWindowEvent, TaoWindowExt};
use library_core::app::get_app;
use library_core::core::{panic_msg, AnyResult, BizError, Exit};
use library_core::file;
use rust_single::{Single, SingleBuild};
use std::path::{Path, PathBuf};
use std::process::exit;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use std::{panic, thread};
use tao::dpi::PhysicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{
    ControlFlow, EventLoop, EventLoopBuilder, EventLoopClosed, EventLoopProxy,
    EventLoopWindowTarget,
};
use tao::window::{Window, WindowBuilder};
use tray_icon::TrayIconEvent;

mod icon;
mod init;
mod tray;
mod window;

pub enum UserEvent {
    EMPTY(),
    NcWindowEvent(NcWindowEvent),
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
            let _ = dispatch(Box::new(|w, _| w.focus_show()));
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

fn main() -> AnyResult<()> {
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
    library_web::init()?;
    let app = get_app();
    let lock_path = app.cache_dir.join("single.lock");
    let mut o_single = create_single(lock_path)?;
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let _proxy = event_loop.create_proxy();
    let mut window = window::Window::new(&event_loop)?;

    library_web::set_open(move |ui| {
        match _proxy.send_event(UserEvent::NcWindowEvent(NcWindowEvent::OpenKernel(ui))) {
            Ok(_) => Ok(()),
            Err(e) => {
                log::error!("发布打开内核界面事件异常! {}", e);
                Err(Box::new(BizError::Unsupported))
            }
        }
    })?;

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(500));
        log::debug!("开始初始化");
        init::start_init();
        log::debug!("初始化完成");
    });

    event_loop.run(move |event, target, control_flow| {
        *control_flow = ControlFlow::Wait;
        let flow = window.on_event(event, target);
        if flow == ControlFlow::Exit {
            let o = o_single.take();
            if let Some(single) = o {
                release_single(single)
            }
            match library_web::stop() {
                Ok(_) => {}
                Err(e) => {
                    log::error!("关闭前停止SingBox异常! {}", e)
                }
            }
        }
        if flow != ControlFlow::Wait {
            *control_flow = flow
        }
    });
}
