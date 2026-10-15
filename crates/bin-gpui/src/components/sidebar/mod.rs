//! 侧栏。
//!
//! 分顶部、中间、底部三段：中间段吃掉剩余高度，因此底部段始终贴在侧栏底边，
//! 不会随着中间内容增减而上下浮动。
//!
//! 侧栏不渲染任何内容，只决定三段的位置与内外边距；每段的子项被撑满内容宽度，
//! 与侧栏边缘之间的距离来自侧栏的 padding。

mod section;

use section::Section;

use gpui_kit::base::{StyledExt as _, v_flex};
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

/// 侧栏宽度。
const SIDEBAR_WIDTH: Pixels = px(220.);

/// 侧栏内边距。
const SIDEBAR_PADDING: Pixels = px(8.);

/// 侧栏。
#[derive(Default, IntoElement)]
pub struct Sidebar {
    top: Vec<AnyElement>,
    middle: Vec<AnyElement>,
    bottom: Vec<AnyElement>,
}

impl Sidebar {
    /// 创建一个三段皆空的侧栏。
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加顶部段的内容。
    ///
    /// 当前应用没有内容放在顶部段，保留它是侧栏三段契约的一部分。
    #[allow(dead_code)]
    pub fn top(mut self, items: impl IntoIterator<Item = impl IntoElement>) -> Self {
        self.top
            .extend(items.into_iter().map(IntoElement::into_any_element));
        self
    }

    /// 追加中间段的内容。中间段占满剩余高度，是导航列表的位置。
    pub fn middle(mut self, items: impl IntoIterator<Item = impl IntoElement>) -> Self {
        self.middle
            .extend(items.into_iter().map(IntoElement::into_any_element));
        self
    }

    /// 追加底部段的内容。
    pub fn bottom(mut self, items: impl IntoIterator<Item = impl IntoElement>) -> Self {
        self.bottom
            .extend(items.into_iter().map(IntoElement::into_any_element));
        self
    }
}

impl RenderOnce for Sidebar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut sidebar = v_flex()
            .w(SIDEBAR_WIDTH)
            .h_full()
            .flex_shrink_0()
            .p(SIDEBAR_PADDING)
            .gap_1()
            .bg(cx.theme().muted)
            .border_r_1()
            .border_color(cx.theme().sidebar_border);

        // 空段不渲染，否则侧栏的段间距会在边缘留下一段空白。
        if !self.top.is_empty() {
            sidebar = sidebar.child(Section::new(self.top));
        }
        if !self.middle.is_empty() {
            sidebar = sidebar.child(Section::new(self.middle).flex_1().min_h_0());
        }
        if !self.bottom.is_empty() {
            sidebar = sidebar.child(Section::new(self.bottom));
        }

        sidebar
    }
}
