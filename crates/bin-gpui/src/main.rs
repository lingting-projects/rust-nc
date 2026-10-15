//! 灵听网络控制。

mod app;
mod assets;
mod blocks;
mod components;
mod nav;
mod theme;
mod utils;

use gpui_kit::*;

use crate::assets::AppAssets;
use crate::blocks::Placeholder;
use crate::nav::Route;

fn main() {
    application().with_assets(AppAssets::new()).run(|cx| {
        init(cx);
        theme::apply(cx);
        nav::init(cx, |router| {
            router.add_route(Route::component("/console", |_, _| {
                Placeholder::new("控制台")
            }));
            router.add_route(Route::component("/node", |_, _| Placeholder::new("节点")));
            router.add_route(Route::component("/connection", |_, _| {
                Placeholder::new("连接")
            }));
            router.add_route(Route::component("/logs", |_, _| Placeholder::new("日志")));
            router.add_route(Route::component("/subscribe", |_, _| {
                Placeholder::new("订阅")
            }));
            router.add_route(Route::component("/rule", |_, _| Placeholder::new("规则")));
            router.add_route(Route::component("/config", |_, _| Placeholder::new("配置")));
            router.add_route(Route::component("/settings", |_, _| {
                Placeholder::new("设置")
            }));
        });
        nav::navigate(cx, app::DEFAULT_PATH);
        app::open(cx).expect("failed to open window");
    });
}
