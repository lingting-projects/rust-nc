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
