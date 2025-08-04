use crate::{icon, UserEvent};
use library_core::core::AnyResult;
use tao::event_loop::EventLoop;
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub fn create(event_loop: &EventLoop<UserEvent>) -> AnyResult<TrayIcon> {
    let proxy = event_loop.create_proxy();
    tray_icon::TrayIconEvent::set_event_handler(Some(move |event| {
        if let Err(e) = proxy.send_event(UserEvent::TrayIconEvent(event)) {
            log::warn!("事件发布异常! {}", e)
        }
    }));

    let proxy = event_loop.create_proxy();
    tray_icon::menu::MenuEvent::set_event_handler(Some(move |event| {
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
