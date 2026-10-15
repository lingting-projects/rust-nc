use gpui_kit::base::StyledExt as _;
use gpui_kit::*;

/// 侧栏中的一段。
///
/// 段本身纵向排列，子项之间留一个基准间距；子项被撑满段的宽度，
/// 即侧栏的内容宽度。
#[derive(IntoElement)]
pub struct Section {
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

impl Section {
    /// 用一组子项创建。
    pub fn new(children: Vec<AnyElement>) -> Self {
        Self {
            children,
            style: StyleRefinement::default(),
        }
    }
}

impl Styled for Section {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Section {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .w_full()
            .gap_1()
            .children(self.children)
            .refine_style(&self.style)
    }
}
