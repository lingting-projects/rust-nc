use gpui_kit::assets::IconName;
use gpui_kit::*;

use crate::blocks::Placeholder;
use crate::pages::Console;
use lingting_gpui_kit::desktop_sidebar_layout::DesktopSidebarLayout;
use lingting_gpui_kit::router::RouteDefinition;

/// 应用标题。
const APP_TITLE: &str = "lingting network control";

/// 应用图标，由 [`crate::assets::AppAssets`] 从磁盘 `assets/icons/` 读取。
const APP_ICON: &str = "icons/256x256.png";

/// 默认打开的页面。
pub const DEFAULT_PATH: &str = "/console";

/// 路由表：既用于注册路由，也用于生成侧栏菜单。
pub fn routes() -> Vec<RouteDefinition> {
    vec![
        RouteDefinition::new("/console", "控制台", |window, cx| {
            Console::new(window, cx)
        })
        .icon(IconName::Gauge),
        RouteDefinition::new("/node", "节点", |_, _| Placeholder::new("节点"))
            .icon(IconName::Server),
        RouteDefinition::new("/connection", "连接", |_, _| Placeholder::new("连接"))
            .icon(IconName::Link),
        RouteDefinition::new("/logs", "日志", |_, _| Placeholder::new("日志"))
            .icon(IconName::ScrollText),
        RouteDefinition::new("/subscribe", "订阅", |_, _| Placeholder::new("订阅"))
            .icon(IconName::Rss),
        RouteDefinition::new("/rule", "规则", |_, _| Placeholder::new("规则"))
            .icon(IconName::GitFork),
        RouteDefinition::new("/config", "配置", |_, _| Placeholder::new("配置"))
            .icon(IconName::FileCog),
        RouteDefinition::new("/settings", "设置", |_, _| Placeholder::new("设置"))
            .icon(IconName::Settings),
    ]
}

/// 打开应用窗口。
pub fn open(
    cx: &mut gpui_kit::App,
    routes: Vec<RouteDefinition>,
) -> Result<(AnyWindowHandle, Entity<DesktopSidebarLayout>)> {
    open_window(window_options(cx), cx, |_, cx| {
        cx.new(|cx| {
            DesktopSidebarLayout::new(cx)
                .routes(routes)
                .icon(|_, _| {
                    img(APP_ICON)
                        .size(px(22.))
                        .rounded_sm()
                        .flex_shrink_0()
                        .into_any_element()
                })
                .title(APP_TITLE)
        })
    })
}

/// 窗口尺寸（物理像素）。
///
/// 与旧实现一致：启动即使用最终尺寸，并把它同时作为最小尺寸。
const WINDOW_SIZE: Size<Pixels> = size(px(1600.), px(1024.));

/// 窗口选项：主屏幕、1600x1024 物理像素、居中、隐藏系统标题栏、保留系统边框。
///
/// GPUI 的 `WindowBounds` 是逻辑像素，窗口实际占用物理像素，两者相差屏幕缩放。
/// 缩放因子在窗口创建前从系统查询，因此这里能一次算出正确的逻辑尺寸。
fn window_options(cx: &gpui_kit::App) -> WindowOptions {
    let scale = crate::utils::primary_scale_factor();
    let size = size(WINDOW_SIZE.width / scale, WINDOW_SIZE.height / scale);

    WindowOptions {
        window_bounds: Some(WindowBounds::centered(size, cx)),
        window_min_size: Some(size),
        titlebar: Some(TitlebarOptions {
            title: Some("lingting network control".into()),
            appears_transparent: true,
            traffic_light_position: None,
        }),
        is_resizable: true,
        is_minimizable: true,
        ..Default::default()
    }
}
