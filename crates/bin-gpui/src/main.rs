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

fn main() {
    application().with_assets(AppAssets::new()).run(|cx| {
        init(cx);
        services::init(cx);
        theme::apply(cx);
        let routes = app::routes();
        lingting_gpui_kit::desktop_sidebar_layout::init_router(cx, app::DEFAULT_PATH, &routes);
        app::open(cx, routes).expect("failed to open window");
    });
}
