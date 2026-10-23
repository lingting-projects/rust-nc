//! 托盘右键菜单。
//!
//! 菜单项在托盘创建时建好并一直复用：`muda` 的菜单项支持改文案与可用性，
//! 内核项按当前运行状态刷新，不重建菜单（重建会让已经弹出的菜单失去句柄）。

use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};

/// 「显示主窗口」菜单项 id。
const SHOW_ID: &str = "tray:show";

/// 「启动/停止内核」菜单项 id。
const KERNEL_ID: &str = "tray:kernel";

/// 「退出」菜单项 id。
const QUIT_ID: &str = "tray:quit";

/// 内核菜单项的文案。
const KERNEL_START: &str = "启动内核";

/// 内核菜单项的文案。
const KERNEL_STOP: &str = "停止内核";

/// 菜单项被点击后要执行的动作。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// 显示并前置主窗口。
    Show,
    /// 切换内核的启停状态。
    ToggleKernel,
    /// 退出应用。
    Quit,
}

/// 托盘菜单。
pub struct TrayMenu {
    menu: Menu,
    kernel: MenuItem,
}

impl TrayMenu {
    /// 创建菜单：显示主窗口、启停内核、退出。
    pub fn new() -> Self {
        let show = MenuItem::with_id(SHOW_ID, "显示主窗口", true, None);
        let kernel = MenuItem::with_id(KERNEL_ID, KERNEL_START, false, None);
        let quit = MenuItem::with_id(QUIT_ID, "退出", true, None);
        let separator = PredefinedMenuItem::separator();

        let menu = Menu::new();
        // 菜单项顺序即界面顺序，分隔线把「退出」与上面的常用操作分开。
        if let Err(error) = menu.append_items(&[&show, &kernel, &separator, &quit]) {
            log::error!("[托盘] 创建菜单失败: {error}");
        }

        Self { menu, kernel }
    }

    /// 交给托盘图标持有的菜单。
    ///
    /// `Menu` 是引用计数的，这里交出的副本与自身共享同一份底层菜单，
    /// 因此后续通过本结构体改文案、可用性时，托盘弹出的菜单会同步生效。
    pub fn context_menu(&self) -> Box<dyn tray_icon::menu::ContextMenu> {
        Box::new(self.menu.clone())
    }

    /// 按内核是否运行刷新菜单项文案与可用性。
    ///
    /// `enabled` 由调用方判断（内核正在启停或没有可用配置时置灰）。
    pub fn refresh_kernel(&self, running: bool, enabled: bool) {
        self.kernel.set_text(match running {
            true => KERNEL_STOP,
            false => KERNEL_START,
        });
        self.kernel.set_enabled(enabled);
    }
}

impl Action {
    /// 由菜单事件解析动作，非本菜单的事件返回 `None`。
    pub fn of(event: &MenuEvent) -> Option<Self> {
        // `MenuId` 与 `&str` 之间实现了比较，直接按字符串匹配即可。
        match event.id.0.as_str() {
            SHOW_ID => Some(Self::Show),
            KERNEL_ID => Some(Self::ToggleKernel),
            QUIT_ID => Some(Self::Quit),
            _ => None,
        }
    }
}
