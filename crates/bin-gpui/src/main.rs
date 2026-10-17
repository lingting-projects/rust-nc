mod assets;
mod blocks;
mod components;
mod pages;
mod services;
mod theme;
mod utils;

use gpui_kit::assets::IconName;
use gpui_kit::*;
use lingting_gpui_kit::WindowOptionsBuilder;
use lingting_gpui_kit::desktop_sidebar_layout::{DesktopSidebarLayout, init_router};
use lingting_gpui_kit::router::RouteDefinition;

use crate::assets::AppAssets;
use crate::blocks::Placeholder;
use crate::pages::{Console, RulePage, SubscribePage};

/// 应用标题。
const APP_TITLE: &str = "lingting network control";

/// 应用图标，由 [`AppAssets`] 从磁盘 `assets/icons/` 读取。
const APP_ICON: &str = "icons/256x256.png";

/// 默认打开的页面。
const DEFAULT_PATH: &str = "/console";

/// 侧栏菜单项高度，随主题字号缩放。
const MENU_HEIGHT: Rems = rems(2.5);

/// 侧栏菜单项图标尺寸，随主题字号缩放。
const MENU_ICON_SIZE: Rems = rems(1.125);

/// 侧栏菜单项字号，随主题字号缩放。
const MENU_TEXT_SIZE: Rems = rems(1.);

fn main() {
    application().with_assets(AppAssets::new()).run(|cx| {
        init(cx);
        services::init(cx);
        theme::apply(cx);

        let routes = routes();
        init_router(cx, DEFAULT_PATH, &routes);
        open(cx, routes).expect("failed to open window");
    });
}

/// 路由表：既用于注册路由，也用于生成侧栏菜单。
fn routes() -> Vec<RouteDefinition> {
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
        RouteDefinition::new("/subscribe", "订阅", |window, cx| {
            SubscribePage::new(window, cx)
        })
        .icon(IconName::Rss),
        RouteDefinition::new("/rule", "规则", RulePage::new).icon(IconName::GitFork),
        RouteDefinition::new("/config", "配置", |_, _| Placeholder::new("配置"))
            .icon(IconName::FileCog),
        RouteDefinition::new("/settings", "设置", |_, _| Placeholder::new("设置"))
            .icon(IconName::Settings),
    ]
}

/// 打开应用窗口。
fn open(
    cx: &mut App,
    routes: Vec<RouteDefinition>,
) -> Result<(AnyWindowHandle, Entity<DesktopSidebarLayout>)> {
    // 窗口尺寸、居中与自绘标题栏所需的设置都取构建器的默认值。
    let options = WindowOptionsBuilder::new().title(APP_TITLE).build();

    open_window(options, cx, |_, cx| {
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
                .menu_height(MENU_HEIGHT)
                .menu_icon_size(MENU_ICON_SIZE)
                .menu_text_size(MENU_TEXT_SIZE)
        })
    })
}
