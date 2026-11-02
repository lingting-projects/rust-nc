//! 系统托盘。
//!
//! 托盘图标由 `tray-icon` 创建，它会在当前线程注册一个隐藏窗口接收 shell 消息，
//! 因此必须在 GPUI 的主线程上初始化，否则消息循环收不到点击与菜单事件。
//!
//! 事件回调来自 shell 线程，不能直接碰 GPUI 状态：点击与菜单事件经通道交回界面线程，
//! 与单进程唤醒的处理方式一致。

#[cfg(target_os = "windows")]
pub(crate) mod close;
#[cfg(target_os = "windows")]
mod kernel;
#[cfg(target_os = "windows")]
mod menu;

#[cfg(target_os = "windows")]
pub use self::imp::{init, ready, refresh, refresh_kernel};

/// 当前平台是否支持托盘。
pub const SUPPORTED: bool = cfg!(target_os = "windows");

/// 关闭窗口时的处理方式。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CloseAction {
    /// 窗口随关闭销毁，进程与内核留在托盘。
    Tray,
    /// 直接退出进程。
    Exit,
}

/// 当前平台的关闭方式。
///
/// 只有 Windows 有托盘，其它平台保持原有的「关闭即退出」。
pub fn close_action() -> CloseAction {
    #[cfg(target_os = "windows")]
    if service_core::holder::settings::SettingsHolder::software().tray {
        return CloseAction::Tray;
    }

    CloseAction::Exit
}

/// 非 Windows 平台的空实现，调用点不需要平台判断。
#[cfg(not(target_os = "windows"))]
mod imp {
    use anyhow::Result;
    use gpui_kit::App;

    /// 当前平台没有托盘：不是错误，只是没有托盘入口。
    pub fn init(_: &mut App) -> Result<bool> {
        Ok(false)
    }

    /// 当前平台没有托盘，没有唤回窗口的入口。
    pub fn ready(_: &App) -> bool {
        false
    }

    /// 当前平台没有托盘，不需要刷新菜单。
    pub fn refresh_kernel(_: &mut App, _: bool, _: bool) {}

    /// 当前平台没有托盘，没有需要刷新的菜单。
    pub fn refresh(_: &mut App) {}
}

#[cfg(target_os = "windows")]
mod imp {
    use anyhow::{Context as _, Result};
    use gpui_kit::*;
    use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
    use tray_icon::menu::MenuEvent;
    use tray_icon::{
        Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    };

    use super::kernel;
    use super::menu::{Action, TrayMenu};
    use crate::assets;

    /// 托盘图标文件，与窗口/任务栏图标同一份资源。
    const ICON: &str = "icons/256x256.ico";

    /// 托盘提示文字。
    const TOOLTIP: &str = "lingting network control";

    /// 托盘图标尺寸，与资源文件内最大尺寸一致。
    const ICON_SIZE: (u32, u32) = (256, 256);

    /// 托盘发出的、需要界面线程处理的事件。
    enum Event {
        /// 左键点击或双击托盘图标。
        Activate,
        /// 右键菜单项被点击。
        Menu(Action),
    }

    /// 托盘全局，界面线程独占持有。
    ///
    /// 图标与菜单句柄必须活到进程退出，否则托盘图标会消失、菜单失效。
    struct Tray {
        _icon: TrayIcon,
        menu: TrayMenu,
        /// 事件发送端，随全局一起存活，界面退出时才释放。
        _events: UnboundedSender<Event>,
    }

    impl Global for Tray {}

    /// 创建托盘图标并开始消费它的事件。
    ///
    /// 图标创建失败时返回错误：托盘是常驻应用的前提，调用方据此退出进程；
    /// 创建成功返回 `true`。
    pub fn init(cx: &mut App) -> Result<bool> {
        let (events, receiver) = unbounded_channel::<Event>();

        let tray_events = events.clone();
        TrayIconEvent::set_event_handler(Some(move |event| {
            if is_activate(&event) {
                send(&tray_events, Event::Activate);
            }
        }));

        let menu_events = events.clone();
        MenuEvent::set_event_handler(Some(move |event| {
            if let Some(action) = Action::of(&event) {
                send(&menu_events, Event::Menu(action));
            }
        }));

        let menu = TrayMenu::new();
        let icon = TrayIconBuilder::new()
            .with_tooltip(TOOLTIP)
            .with_icon(icon()?)
            // 左键用于唤回窗口，菜单交给右键
            .with_menu_on_left_click(false)
            .with_menu(menu.context_menu())
            .build()
            .context("创建托盘图标失败")?;

        cx.set_global(Tray {
            _icon: icon,
            menu,
            _events: events,
        });

        listen(cx, receiver);
        kernel::refresh(cx);
        Ok(true)
    }

    /// 托盘是否可用。
    ///
    /// 没有托盘就没有唤回窗口的入口，窗口关闭后必须退出而不是留在后台。
    pub fn ready(cx: &App) -> bool {
        cx.try_global::<Tray>().is_some()
    }

    /// 按内核的当前状态刷新菜单项。
    pub fn refresh(cx: &mut App) {
        kernel::refresh(cx);
    }

    /// 直接刷新菜单项。
    pub fn refresh_kernel(cx: &mut App, running: bool, enabled: bool) {
        if let Some(tray) = cx.try_global::<Tray>() {
            tray.menu.refresh_kernel(running, enabled);
        }
    }

    /// 在界面线程上消费托盘事件。
    fn listen(cx: &mut App, mut receiver: UnboundedReceiver<Event>) {
        cx.spawn(async move |cx| {
            while let Some(event) = receiver.recv().await {
                cx.update(|cx| handle(cx, event));
            }
        })
        .detach();
    }

    /// 处理一个托盘事件。
    fn handle(cx: &mut App, event: Event) {
        match event {
            // 窗口可能已经被销毁：唤回时按需重建
            Event::Activate | Event::Menu(Action::Show) => crate::main_window::show_or_open(cx),
            Event::Menu(Action::Quit) => {
                log::info!("[退出] 托盘菜单触发退出");
                cx.quit();
            }
            Event::Menu(Action::ToggleKernel) => kernel::toggle(cx),
        }
    }

    /// 托盘图标。
    fn icon() -> anyhow::Result<Icon> {
        let path = assets::root().join(ICON);
        Ok(Icon::from_path(&path, Some(ICON_SIZE))?)
    }

    /// 是否是需要唤回窗口的点击。
    ///
    /// 左键按下时不处理：Windows 在按下与抬起之间还会发别的消息，抬起才算一次点击。
    fn is_activate(event: &TrayIconEvent) -> bool {
        match event {
            TrayIconEvent::Click {
                button,
                button_state,
                ..
            } => *button == MouseButton::Left && *button_state == MouseButtonState::Up,
            TrayIconEvent::DoubleClick { button, .. } => *button == MouseButton::Left,
            _ => false,
        }
    }

    /// 把事件交给界面线程。
    fn send(sender: &UnboundedSender<Event>, event: Event) {
        if sender.send(event).is_err() {
            log::error!("[托盘] 事件转交失败: 界面已退出");
        }
    }
}
