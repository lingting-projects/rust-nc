//! 标题栏左侧：应用图标 + 标题。

use gpui_kit::base::StyledExt as _;
use gpui_kit::*;

/// 应用标题。
const APP_TITLE: &str = "lingting network control";

/// 应用图标，由 [`crate::assets::AppAssets`] 从磁盘 `assets/icons/` 读取。
const APP_ICON: &str = "icons/256x256.png";

/// 左侧区域。整体作为窗口拖动区。
pub fn left() -> AnyElement {
    div()
        .id("title-bar-left")
        .flex()
        .flex_row()
        .items_center()
        .gap_2()
        .pl_3()
        .h_full()
        .flex_shrink_0()
        .window_control_area(WindowControlArea::Drag)
        .child(img(APP_ICON).size(px(22.)).rounded_sm().flex_shrink_0())
        .child(div().text_sm().font_semibold().child(APP_TITLE))
        .into_any_element()
}
