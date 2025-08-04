use crate::window::WindowManager;
use crate::{icon, UserEvent};
use library_core::core::AnyResult;
use tao::event_loop::EventLoop;
use tray_icon::menu::MenuEvent;
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};

pub fn create(event_loop: &EventLoop<UserEvent>) -> AnyResult<TrayIcon> {
    let proxy = event_loop.create_proxy();
    TrayIconEvent::set_event_handler(Some(move |event| {
        if let Err(e) = proxy.send_event(UserEvent::TrayIconEvent(event)) {
            log::warn!("事件发布异常! {}", e)
        }
    }));

    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event| {
        if let Err(e) = proxy.send_event(UserEvent::MenuEvent(event)) {
            log::warn!("事件发布异常! {}", e)
        }
    }));

    let tray = TrayIconBuilder::new()
        .with_title("lingting-nc")
        .with_tooltip("lingting network control")
        .with_icon(Icon::from_path(
            icon::path,
            Some((icon::width, icon::height)),
        )?)
        .build()?;

    Ok(tray)
}

pub fn handler_icon(manager: &WindowManager, e: TrayIconEvent) {
    match e {
        TrayIconEvent::Click { .. } => {
            manager.show();
        }
        _ => {}
    }
}

pub fn handler_menu(manager: &WindowManager, e: MenuEvent) {
    // 比对事件id, 实现对应的事件
}
