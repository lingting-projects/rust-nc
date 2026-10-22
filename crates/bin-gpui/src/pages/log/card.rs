//! 日志块卡片。
//!
//! 每块占两行：上行是等级、首行时间与未展示内容的提示，下行是首行文本。
//! 高度固定，因此虚拟列表可以直接按固定行高排布。

use gpui_kit::base::{StyledExt as _, h_flex};
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

use lingting_gpui_kit::card::ItemCard;

use lib_singbox::{LogBlock, LogLevel};

use super::LogPage;

/// 日志块卡片高度。
///
/// 上下两行各一个行高，再加卡片自身的垂直留白，取值略大于两行文字高度。
pub const ITEM_HEIGHT: Rems = rems(3.25);

/// 标签列宽度：等级文本长度固定，统一宽度让首行时间与正文对齐。
const LEVEL_WIDTH: Rems = rems(3.25);

/// 渲染一个日志块卡片。
pub fn render(page: &Entity<LogPage>, block: &LogBlock, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let (color, level) = match block.level {
        LogLevel::Panic | LogLevel::Fatal | LogLevel::Error => (theme.danger, block.level.label()),
        LogLevel::Warn => (theme.warning, block.level.label()),
        LogLevel::Info => (theme.foreground, block.level.label()),
        LogLevel::Debug | LogLevel::Trace => (theme.muted_foreground, block.level.label()),
        // 续行与无法识别的前缀没有等级文本，只在续行上显示行数提示
        LogLevel::Unknown => (theme.muted_foreground, ""),
    };

    let hint = block.hint();
    let top = h_flex()
        .w_full()
        .gap_2()
        .child(
            div()
                .w(LEVEL_WIDTH)
                .flex_shrink_0()
                .font_weight(FontWeight::MEDIUM)
                .text_color(color)
                .child(level),
        )
        .child(div().flex_1().min_w_0())
        // 块大小：详情会整块完整读出，块越大成本越高，交由用户判断是否值得展开
        .child(
            div()
                .flex_shrink_0()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(block.size()),
        )
        .child(
            div()
                .flex_shrink_0()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(hint),
        );

    let bottom = div()
        .w_full()
        .min_w_0()
        .truncate()
        .text_sm()
        .child(block.text.to_string());

    let card = ItemCard::new(("log-block", block.start))
        .with_center_split(top, bottom)
        // 只有块内还有未展示的内容时才可点击，避免弹出与卡片完全相同的窗口
        .hoverable(block.expandable());

    if !block.expandable() {
        return card.into_any_element();
    }

    let entity = page.clone();
    let start = block.start;
    card.on_click(move |window, cx| {
        entity.update(cx, |page, cx| page.open_detail(start, window, cx));
    })
    .into_any_element()
}
