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
            .child(div().child(self.title.clone()))
            .child(
                div()
                    .text_color(cx.theme().muted_foreground)
                    .child("页面开发中。"),
            )
    }
}
