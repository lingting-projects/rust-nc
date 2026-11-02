//! 主窗口的创建、重建与显隐。
//!
//! 窗口只是一个随时可以丢掉的前端：关闭后进程、内核与托盘继续运行，需要时再从托盘唤回时按需重建。
//! 这样显示配置变化、显卡切换这类系统事件即使摧毁窗口，也不会连带结束整个应用。
//!
//! 路由表在这里注册并持有：`init_router` 整个进程只能执行一次，重建窗口只重建根视图，
//! 页面实体与当前路由都是全局状态，重建后照旧。

use anyhow::Result;
use gpui_kit::assets::IconName;
use gpui_kit::*;
use lingting_gpui_kit::WindowOptionsBuilder;
use lingting_gpui_kit::desktop_sidebar_layout::{DesktopSidebarLayout, init_router};
use lingting_gpui_kit::router::RouteDefinition;

use crate::pages::{
    ConfigPage, ConnectionPage, Console, LogPage, NodePage, RulePage, SettingsPage, SubscribePage,
};
use crate::tray;
use crate::utils::window::window_state;

/// 应用标题。
const APP_TITLE: &str = "lingting network control";

/// 应用图标，由 [`AppAssets`](crate::assets::AppAssets) 从磁盘 `assets/icons/` 读取。
const APP_ICON: &str = "icons/256x256.png";

/// 默认打开的页面。
const DEFAULT_PATH: &str = "/console";

/// 侧栏菜单项高度，随主题字号缩放。
const MENU_HEIGHT: Rems = rems(2.5);

/// 侧栏菜单项图标尺寸，随主题字号缩放。
const MENU_ICON_SIZE: Rems = rems(1.125);

/// 侧栏菜单项字号，随主题字号缩放。
const MENU_TEXT_SIZE: Rems = rems(1.);

/// 注册路由表。
///
/// 必须在创建窗口之前调用，且整个进程只调用一次；重建窗口不需要再注册。
pub fn init(cx: &mut App) {
    init_router(cx, DEFAULT_PATH, &routes());
}

/// 创建主窗口。
///
/// `hidden` 为真时窗口直接以隐藏状态创建，先显示再收起会闪一下。
pub fn open(cx: &mut App, hidden: bool) -> Result<AnyWindowHandle> {
    // 窗口尺寸、居中与自绘标题栏所需的设置都取构建器的默认值。
    let mut options = WindowOptionsBuilder::new().title(APP_TITLE).build();
    // 构建器没有对应方法，这里直接改选项：为 `false` 时窗口创建时不调用
    // `SetWindowPlacement`，只在内部记下初始位置，之后第一次显示才真正摆好。
    options.show = !hidden;

    let (window, _) = open_window(options, cx, |_, cx| {
        cx.new(|cx| {
            DesktopSidebarLayout::new(cx)
                .routes(routes())
                .icon(|_, _| {
                    img(APP_ICON)
                        .size(px(22.))
                        .rounded_sm()
                        .flex_shrink_0()
                        .into_any_element()
                })
                .title(APP_TITLE)
                .menu_height(MENU_HEIGHT)
                .menu_icon_size(MENU_ICON_SIZE)
                .menu_text_size(MENU_TEXT_SIZE)
        })
    })?;

    // 关闭按钮的语义由设置决定。只有 Windows 有托盘，其它平台没有唤回入口，关闭窗口就是退出。
    #[cfg(target_os = "windows")]
    tray::close::install(cx, window);

    // 窗口状态必须在页面创建之前就位：隐藏启动时页面会直接挂起轮询。
    window_state().set_visible(!hidden);
    Ok(window)
}

/// 唤回主窗口：窗口还在就显示并前置，已经被销毁就重建。
///
/// 托盘点击、托盘菜单与后续实例的唤醒信号都走这里，调用方不需要持有窗口句柄。
pub fn show_or_open(cx: &mut App) {
    match current(cx) {
        Some(window) => show(cx, window),
        None => {
            log::info!("[窗口] 主窗口不存在, 重新创建");
            if let Err(error) = open(cx, false) {
                log::error!("[窗口] 重建主窗口失败: {error:#}");
            }
        }
    }
}

/// 显示并前置主窗口。
pub fn show(cx: &mut App, window: AnyWindowHandle) {
    let result = window.update(cx, |_, window, cx| {
        // 隐藏创建的窗口要等 `activate` 里应用初始位置时才真正显示出来
        window.activate_window();
        cx.activate(true);
        // 先让窗口真正回到眼前再置位：轮询恢复后会立刻刷新一次，此时窗口已经可见
        window_state().set_visible(true);
    });
    if let Err(error) = result {
        log::error!("[窗口] 显示主窗口失败: {error:#}");
    }
}

/// 当前主窗口。
///
/// 应用只开一个窗口，因此 `cx.windows()` 里最多只有一个；窗口销毁后为空。
fn current(cx: &App) -> Option<AnyWindowHandle> {
    cx.windows().into_iter().next()
}

/// 路由表：既用于注册路由，也用于生成侧栏菜单。
fn routes() -> Vec<RouteDefinition> {
    vec![
        RouteDefinition::new("/console", "控制台", |window, cx| {
            Console::new(window, cx)
        })
        .icon(IconName::Gauge),
        RouteDefinition::new("/node", "节点", NodePage::new).icon(IconName::Server),
        RouteDefinition::new("/connection", "连接", ConnectionPage::new).icon(IconName::Link),
        RouteDefinition::new("/logs", "日志", LogPage::new).icon(IconName::ScrollText),
        RouteDefinition::new("/subscribe", "订阅", |window, cx| {
            SubscribePage::new(window, cx)
        })
        .icon(IconName::Rss),
        RouteDefinition::new("/rule", "规则", RulePage::new).icon(IconName::GitFork),
        RouteDefinition::new("/config", "配置", ConfigPage::new).icon(IconName::FileCog),
        RouteDefinition::new("/settings", "设置", SettingsPage::new).icon(IconName::Settings),
    ]
}
