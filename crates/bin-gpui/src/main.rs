//! 灵听网络控制。

mod app;
mod assets;
mod blocks;
mod components;
mod theme;
mod utils;

use gpui_kit::*;

use crate::assets::AppAssets;
use crate::blocks::Placeholder;

fn main() {
    application().with_assets(AppAssets::new()).run(|cx| {
        init(cx);
        theme::apply(cx);
        lingting_gpui_kit::init(cx, |router| {
            router.add_cache("/console", |_, _| Placeholder::new("控制台"));
            router.add_cache("/node", |_, _| Placeholder::new("节点"));
            router.add_cache("/connection", |_, _| Placeholder::new("连接"));
            router.add_cache("/logs", |_, _| Placeholder::new("日志"));
            router.add_cache("/subscribe", |_, _| Placeholder::new("订阅"));
            router.add_cache("/rule", |_, _| Placeholder::new("规则"));
            router.add_cache("/config", |_, _| Placeholder::new("配置"));
            router.add_cache("/settings", |_, _| Placeholder::new("设置"));
        });
        lingting_gpui_kit::navigate(cx, app::DEFAULT_PATH);
        app::open(cx).expect("failed to open window");
    });
}
