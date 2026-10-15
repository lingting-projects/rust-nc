//! 单个菜单项。

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::nav;

/// 一个菜单项。
pub struct MenuItem {
    /// 路由路径，同时作为选中判定的标识。
    pub path: &'static str,
    pub label: &'static str,
    /// `assets/icons/` 下的 SVG 路径。
    pub icon: &'static str,
}

/// 图标尺寸。
const ICON_SIZE: Pixels = px(14.);

/// 菜单项高度。
const ITEM_HEIGHT: Pixels = px(32.);

/// 选中项的投影。
///
/// GPUI 的 `blur_radius` 是高斯标准差，而 CSS 的 blur 是它的两倍，
/// 照搬 Tailwind 的半径会糊成一片灰雾，因此这里取一半的半径。
fn selected_shadow() -> Vec<BoxShadow> {
    let ink = hsla(0., 0., 0., 0.1);
    vec![
        BoxShadow::new(px(0.), px(1.), ink).blur_radius(px(1.5)),
        BoxShadow::new(px(0.), px(2.), ink)
            .blur_radius(px(2.))
            .spread_radius(px(-1.)),
    ]
}

/// 渲染一个菜单项。
///
/// 本身不设置宽度：放进侧栏的满宽槽位后占满整行。
pub fn menu(item: &MenuItem, selected: bool, cx: &App) -> AnyElement {
    // SVG 不继承文本颜色，必须显式给值，否则图标不绘制。
    let icon_color = cx.theme().foreground;
    let path = item.path;

    div()
        .id(item.path)
        .flex()
        .items_center()
        .gap_2()
        .px_4()
        .h(ITEM_HEIGHT)
        .rounded_md()
        .text_sm()
        .cursor_pointer()
        .when(selected, |this| {
            // 底 + 一圈浅边 + 投影，视觉上从侧栏底色里浮起来
            this.bg(cx.theme().background)
                .text_color(cx.theme().foreground)
                .border_1()
                .border_color(cx.theme().border)
                .shadow(selected_shadow())
        })
        .when(!selected, |this| {
            this.hover(|this| this.bg(cx.theme().secondary_hover))
        })
        .on_click(move |_, _, cx| nav::navigate(cx, path))
        .child(
            svg()
                .path(item.icon)
                .size(ICON_SIZE)
                .text_color(icon_color),
        )
        .child(item.label)
        .into_any_element()
}
