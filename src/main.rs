mod core;
mod init;
mod window;
mod app;

use crate::init::start_init;
use crate::window::WindowManager;
use log::LevelFilter;
use simple_logger::SimpleLogger;
use std::process::exit;
use std::thread;
use std::time::Duration;
use tao::event_loop::{ControlFlow, EventLoop};
use thiserror::__private::AsDynError;
use wry::cookie::time;

use time::{format_description::FormatItem, macros::format_description};

const TIMESTAMP_FORMAT: &[FormatItem] = format_description!(
    "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"
);

fn main() {
    app::init();

    let logger = SimpleLogger::new()
        .with_local_timestamps()
        .with_timestamp_format(TIMESTAMP_FORMAT)
        .with_level(LevelFilter::Debug);
    logger.init().unwrap();
    let event_loop = EventLoop::new();
    let mut manager = WindowManager::new(&event_loop);

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(500));
        log::debug!("开始初始化");
        match start_init() {
            Ok(_) => {
                log::debug!("初始化完成")
            }
            Err(e) => {
                log::error!("初始化异常! {}", e.as_dyn_error());
                exit(2)
            }
        }
    });

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        manager.handle_event(&event, control_flow);
    });
}
