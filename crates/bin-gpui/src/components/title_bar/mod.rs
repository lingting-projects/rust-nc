//! 自绘标题栏。
//!
//! 布局为 左侧（图标 + 标题） - 中间（空白拖动区） - 右侧（窗口按钮）。
//!
//! 可拖动区域必须避开交互元素：`WindowControlArea` 的命中判定按插入顺序取第一个，
//! 父元素上的 `Drag` 会盖住子元素的窗口按钮，因此 `Drag` 只加在左侧与中间空白上。

mod controls;
mod left;

pub use controls::window_controls;
pub use left::left;

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

/// 标题栏高度。
pub const TITLE_BAR_HEIGHT: Pixels = px(52.);

/// 渲染标题栏。
pub fn title_bar(window: &Window, cx: &App) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .w_full()
        .h(TITLE_BAR_HEIGHT)
        .flex_shrink_0()
        .bg(cx.theme().title_bar)
        .border_b_1()
        .border_color(cx.theme().title_bar_border)
        .child(left())
        .child(drag_spacer())
        .child(window_controls(window, cx))
        .into_any_element()
}

/// 左侧与窗口按钮之间的空白拖动区。
fn drag_spacer() -> AnyElement {
    div()
        .id("title-bar-drag")
        .flex_1()
        .h_full()
        .window_control_area(WindowControlArea::Drag)
        .into_any_element()
}
