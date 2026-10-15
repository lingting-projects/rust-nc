//! 应用外壳：自绘标题栏 + 左侧侧栏 + 页面内容区。

use gpui_kit::component::{ActiveTheme as _, WindowExt as _};
use gpui_kit::*;

use crate::components::{MenuItem, Sidebar, menu, title_bar};
use crate::nav::{Outlet, current_path};

/// 渲染在侧栏中间段的菜单项。
const MENU_ITEMS: &[MenuItem] = &[
    MenuItem {
        path: "/console",
        label: "控制台",
        icon: "assets/icons/gauge.svg",
    },
    MenuItem {
        path: "/node",
        label: "节点",
        icon: "assets/icons/server.svg",
    },
    MenuItem {
        path: "/connection",
        label: "连接",
        icon: "assets/icons/link.svg",
    },
    MenuItem {
        path: "/logs",
        label: "日志",
        icon: "assets/icons/scroll-text.svg",
    },
    MenuItem {
        path: "/subscribe",
        label: "订阅",
        icon: "assets/icons/rss.svg",
    },
    MenuItem {
        path: "/rule",
        label: "规则",
        icon: "assets/icons/git-fork.svg",
    },
    MenuItem {
        path: "/config",
        label: "配置",
        icon: "assets/icons/file-cog.svg",
    },
    MenuItem {
        path: "/settings",
        label: "设置",
        icon: "assets/icons/settings.svg",
    },
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(title_bar(window, cx))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .min_h_0()
                    .child(Sidebar::new().middle(nav_list(MENU_ITEMS, cx)))
                    .child(div().flex_1().min_w_0().child(self.outlet.clone())),
            )
    }
}

/// 把一组菜单项渲染成侧栏的导航列表。
///
/// 每项一个满宽槽位，选中态取自当前路由。
fn nav_list(items: &[MenuItem], cx: &App) -> Vec<AnyElement> {
    let current = current_path(cx);

    items
        .iter()
        .map(|item| {
            let selected = item.path == current.as_ref();
            div()
                .w_full()
                .child(menu(item, selected, cx))
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
