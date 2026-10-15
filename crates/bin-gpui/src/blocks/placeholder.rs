//! 占位页面。
//!
//! 页面内容尚未实现时的临时页面：只显示页面名称，用于验证布局与路由。

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

pub struct Placeholder {
    title: SharedString,
}

impl Placeholder {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
        }
    }
}

impl Render for Placeholder {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_6()
            .child(div().text_xl().child(self.title.clone()))
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("页面开发中。"),
            )
    }
}
