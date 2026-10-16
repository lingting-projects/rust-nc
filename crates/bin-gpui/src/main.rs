mod app;
mod assets;
mod blocks;
mod components;
mod pages;
mod services;
mod theme;
mod utils;

use gpui_kit::*;

use crate::assets::AppAssets;
use crate::blocks::Placeholder;

fn main() {
    application().with_assets(AppAssets::new()).run(|cx| {
        init(cx);
        services::init(cx);
        theme::apply(cx);
        lingting_gpui_kit::init(cx, |router| {
            // 控制台需要自己的 `Context` 持有下拉框状态，因此用已擦除工厂注册。
            router.add_erased(
                "/console",
                std::rc::Rc::new(|window, cx| cx.new(|cx| pages::Console::new(window, cx)).into()),
                true,
            );
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
