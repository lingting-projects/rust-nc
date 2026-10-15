//! 应用外壳：自绘标题栏 + 左侧侧栏 + 页面内容区。

use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

use lingting_gpui_kit::menu::Menu;
use lingting_gpui_kit::title_bar::TitleBar;
use lingting_gpui_kit::{Outlet, current_path};

use crate::components::Sidebar;

/// 应用标题。
const APP_TITLE: &str = "lingting network control";

/// 应用图标，由 [`crate::assets::AppAssets`] 从磁盘 `assets/icons/` 读取。
const APP_ICON: &str = "icons/256x256.png";

/// 渲染在侧栏中间段的菜单项：路径、名称、图标。
const MENU_ITEMS: &[(&str, &str, IconName)] = &[
    ("/console", "控制台", IconName::Gauge),
    ("/node", "节点", IconName::Server),
    ("/connection", "连接", IconName::Link),
    ("/logs", "日志", IconName::ScrollText),
    ("/subscribe", "订阅", IconName::Rss),
    ("/rule", "规则", IconName::GitFork),
    ("/config", "配置", IconName::FileCog),
    ("/settings", "设置", IconName::Settings),
];

/// 默认打开的页面。
pub const DEFAULT_PATH: &str = "/console";

/// 窗口内容。
pub struct Shell {
    outlet: Entity<Outlet>,
}

impl Shell {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            outlet: cx.new(Outlet::new),
        }
    }
}

impl Render for Shell {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(
                TitleBar::new()
                    .icon(img(APP_ICON).size(px(22.)).rounded_sm().flex_shrink_0())
                    .title(APP_TITLE),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .min_h_0()
                    .child(Sidebar::new().middle(nav_list(cx)))
                    .child(div().flex_1().min_w_0().child(self.outlet.clone())),
            )
    }
}

/// 把一组菜单项渲染成侧栏的导航列表。
///
/// 每项一个满宽槽位，选中态取自当前路由。
fn nav_list(cx: &App) -> Vec<AnyElement> {
    let current = current_path(cx);

    MENU_ITEMS
        .iter()
        .map(|(path, label, icon)| {
            let selected = *path == current.as_ref();
            div()
                .w_full()
                .child(Menu::new(*path, *label, *icon).selected(selected))
                .into_any_element()
        })
        .collect()
}

/// 打开应用窗口。
pub fn open(cx: &mut gpui_kit::App) -> Result<(AnyWindowHandle, Entity<Shell>)> {
    open_window(window_options(cx), cx, |_, cx| cx.new(Shell::new))
}

/// 窗口尺寸（物理像素）。
///
/// 与旧实现一致：启动即使用最终尺寸，并把它同时作为最小尺寸。
const WINDOW_SIZE: Size<Pixels> = size(px(1600.), px(1024.));

/// 窗口选项：主屏幕、1620x810 物理像素、居中、隐藏系统标题栏、保留系统边框。
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
