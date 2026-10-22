//! 日志块卡片。
//!
//! 每块占两行：上行是等级、时间、块大小与未展示内容的提示，下行是首行正文。
//! 高度固定，因此虚拟列表可以直接按固定行高排布。

use gpui_kit::base::{StyledExt as _, h_flex, v_flex};
use gpui_kit::component::{ActiveTheme as _, Theme};
use gpui_kit::*;

use lingting_gpui_kit::card::ItemCard;

use lib_singbox::{LogBlock, LogLevel};

use super::LogPage;

/// 日志块卡片高度。
///
/// 上下两行各一个行高，再加卡片自身的垂直留白，取值略大于两行文字高度。
/// 下行的正文比上行多留出空间，多出来的高度落在卡片下部。
pub const ITEM_HEIGHT: Rems = rems(4.);

/// 标签列宽度：等级文本长度固定，统一宽度让时间与正文对齐。
const LEVEL_WIDTH: Rems = rems(3.75);

/// 底部内置间距。
const PAD_BOTTOM: Rems = rems(0.25);

/// 等级对应的文本颜色。
///
/// 等级是日志里最需要扫读的分类，按主题语义色区分严重程度：
/// 越严重越接近 danger，越详细越淡，避免整屏同色。
fn level_color(level: LogLevel, theme: &Theme) -> Hsla {
    match level {
        LogLevel::Panic | LogLevel::Fatal | LogLevel::Error => theme.danger,
        LogLevel::Warn => theme.warning,
        LogLevel::Info => theme.info,
        // 更详细的等级不参与告警语义，退回次要文字色，避免整屏都是彩色
        LogLevel::Debug | LogLevel::Trace | LogLevel::Unknown => theme.muted_foreground,
    }
}

/// 渲染一个日志块卡片。
pub fn render(page: &Entity<LogPage>, block: &LogBlock, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let muted = theme.muted_foreground;

    let top = h_flex()
        .w_full()
        .gap_2()
        .child(
            div()
                .w(LEVEL_WIDTH)
                .flex_shrink_0()
                .font_weight(FontWeight::MEDIUM)
                .text_color(level_color(block.level, theme))
                .child(block.level.label()),
        )
        // 时间已从正文里剔除，只在元信息行展示，正文因此从同一起点开始；
        // 这一栏吃掉剩余宽度，窗口变窄时先截断时间而不是挤掉块大小与续行提示
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_xs()
                .text_color(muted)
                .child(block.time.to_string()),
        )
        // 块大小：详情会整块完整读出，块越大成本越高，交由用户判断是否值得展开
        .child(
            div()
                .flex_shrink_0()
                .text_xs()
                .text_color(muted)
                .child(block.size()),
        )
        .child(
            div()
                .flex_shrink_0()
                .text_xs()
                .text_color(muted)
                .child(block.hint()),
        );

    let bottom = div()
        .w_full()
        .min_w_0()
        .truncate()
        .text_sm()
        .child(block.text.to_string());

    let card = ItemCard::new(("log-block", block.start))
        // 卡片下半部分留白更多：元信息行保持紧凑，正文行拿掉多出来的高度，
        // 两行文字仍各自贴在自己那一块的上沿。
        .with_center_split(top, v_flex().h_full().pb(PAD_BOTTOM).child(bottom))
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
