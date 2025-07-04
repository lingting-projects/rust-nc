use crate::window::WindowManager;
use std::process::exit;
use std::thread;
use std::time::Duration;
use tao::event_loop::{ControlFlow, EventLoop};

mod init;
mod window;

fn main() {
    library_core::app::init();
    let event_loop = EventLoop::new();
    let mut manager = WindowManager::new(&event_loop);

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(500));
        log::debug!("开始初始化");
        match init::start_init() {
            Ok(_) => {
                log::debug!("初始化完成")
            }
            Err(e) => {
                match e.source() {
                    None => {
                        log::error!("初始化异常! 未知异常!");
                    }
                    Some(e) => {
                        log::error!("初始化异常! {}", e);
                    }
                }
                exit(2)
            }
        }
    });

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        manager.handle_event(&event, control_flow);
    });
}
