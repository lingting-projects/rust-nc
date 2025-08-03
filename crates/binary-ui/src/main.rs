use crate::window::WindowManager;
use library_core::core::AnyResult;
use std::thread;
use std::time::Duration;
use tao::event_loop::{ControlFlow, EventLoop};

mod init;
mod window;
mod uiview;

#[tokio::main]
async fn main() -> AnyResult<()> {
    library_core::app::init()?;
    let event_loop = EventLoop::new();
    let mut manager = WindowManager::new(&event_loop);

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(500));
        log::debug!("开始初始化");
        init::start_init();
        log::debug!("初始化完成");
    });

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        manager.handle_event(&event, control_flow);
    });
}
