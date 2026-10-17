use gpui_kit::assets::IconName;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants as _;
use gpui_kit::component::{ActiveTheme as _, Sizable as _};
use gpui_kit::*;

/// 表格行内的删除按钮。
///
/// 保持 ghost 的轻量观感，只用危险色标记不可撤销的操作；
/// 图标与提示文案由调用方给出，保证同一操作在各页面里是同一种样式。
pub fn delete_button(
    id: impl Into<ElementId>,
    icon: IconName,
    label: &'static str,
    cx: &App,
) -> Button {
    Button::new(id)
        .ghost()
        .small()
        .icon(icon)
        .text_color(cx.theme().danger)
        .tooltip(label)
        .accessibility_label(label)
}
