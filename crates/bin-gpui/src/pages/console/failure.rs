//! 启动失败日志卡片。
//!
//! 内核启动失败时，失败原因里已经带上内核日志尾部的内容，
//! 因此这里直接把原因完整展开，不再另读日志文件。

use gpui_kit::base::{StyledExt as _, h_flex, v_flex};
use gpui_kit::component::{ActiveTheme as _, Icon, IconName};
use gpui_kit::*;

use lingting_gpui_kit::card::Card;

use super::Console;
use super::PADDING;

/// 卡片标题。
const TITLE: &str = "启动失败";

/// 渲染启动失败日志。
///
/// 只在「未运行且记录了失败原因」时渲染：内核运行中或从未失败过都没有内容可展示。
/// 卡片高度随日志长度展开，超出窗口时由控制台页面整体滚动。
pub fn render(console: &Console, cx: &App) -> Option<AnyElement> {
    if console.kernel.running() {
        return None;
    }
    let reason = console.kernel.reason.as_ref()?;
    let theme = cx.theme();

    Some(
        Card::new()
            .w_full()
            .flex_shrink_0()
            .content_padding(PADDING)
            .child(
                v_flex()
                    .w_full()
                    .gap_3()
                    .child(
                        h_flex()
                            .gap_2()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.danger)
                            .child(Icon::new(IconName::CircleX).text_color(theme.danger))
                            .child(TITLE),
                    )
                    .child(
                        div()
                            .w_full()
                            .text_sm()
                            // 正文用等宽字体保留内核原文的缩进与堆栈对齐；
                            // 颜色仍用常规文字色，红色只留给标题。
                            .font_family(theme.mono_font_family.clone())
                            .child(reason.clone()),
                    ),
            )
            .into_any_element(),
    )
}
