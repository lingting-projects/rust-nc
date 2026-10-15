//! 窗口控制按钮（最小化 / 最大化 / 关闭）。
//!
//! 仅 Windows 渲染。按钮通过 [`WindowControlArea`] 交给系统处理点击，
//! 因此不需要自己绑定点击事件。

use gpui_kit::assets::IconName;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

/// 单个按钮宽度。
const BUTTON_WIDTH: Pixels = px(46.);

/// 图标尺寸。
const ICON_SIZE: Pixels = px(14.);

/// 窗口控制按钮组。
pub fn window_controls(window: &Window, cx: &App) -> AnyElement {
    if !cfg!(target_os = "windows") {
        return div().into_any_element();
    }

    let maximize_icon = if window.is_maximized() {
        IconName::WindowRestore
    } else {
        IconName::WindowMaximize
    };
    let foreground = cx.theme().foreground;

    div()
        .flex()
        .flex_row()
        .items_center()
        .h_full()
        .flex_shrink_0()
        .child(button(
            "window-minimize",
            IconName::WindowMinimize,
            WindowControlArea::Min,
            cx.theme().secondary_hover,
            foreground,
        ))
        .child(button(
            "window-maximize",
            maximize_icon,
            WindowControlArea::Max,
            cx.theme().secondary_hover,
            foreground,
        ))
        .child(button(
            "window-close",
            IconName::WindowClose,
            WindowControlArea::Close,
            cx.theme().danger,
            foreground,
        ))
        .into_any_element()
}

/// 一个窗口控制按钮。
///
/// 颜色必须显式设在 `svg()` 自身：SVG 元素取的是元素自己的文本颜色，
/// 既不继承父元素，缺省时也不会绘制。
fn button(
    id: &'static str,
    icon: IconName,
    area: WindowControlArea,
    hover_bg: Hsla,
    icon_color: Hsla,
) -> AnyElement {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .w(BUTTON_WIDTH)
        .h_full()
        .flex_shrink_0()
        .cursor_pointer()
        .hover(move |this| this.bg(hover_bg))
        .window_control_area(area)
        .child(
            svg()
                .path(icon.path())
                .size(ICON_SIZE)
                .text_color(icon_color),
        )
        .into_any_element()
}
