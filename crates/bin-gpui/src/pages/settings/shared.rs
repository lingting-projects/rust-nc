//! 设置页内核分区共用的行与卡片。
//!
//! 内核设置与内核配置两个分区都按「左侧标签 + 右侧内容」排列，
//! 共用同一套列宽与行距，避免两个卡片的内容起始线错位。

use gpui_kit::base::{StyledExt as _, h_flex, v_flex};
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::*;

use lingting_gpui_kit::card::Card;

/// 卡片内边距，与其它页面保持一致。
const PADDING: Rems = rems(1.);

/// 标签列宽度：固定后各行内容从同一条起始线开始。
const LABEL_WIDTH: Rems = rems(5.);

/// 标签与内容之间的间隔。
const COLUMN_GAP: Rems = rems(1.);

/// 各行之间的间隔。
pub const ROW_GAP: Rems = rems(0.75);

/// 只读项的占位，表示该项没有可用取值。
const EMPTY: &str = "未配置";

/// 分区卡片：统一标题与内边距。
pub fn card(title: &'static str, content: AnyElement) -> impl IntoElement {
    Card::new()
        .w_full()
        .flex_shrink_0()
        .title_padding(PADDING)
        .content_padding(PADDING)
        .title(div().font_weight(FontWeight::MEDIUM).child(title))
        .child(content)
}

/// 分区状态提示，失败时用危险色。
pub fn message(reason: &str, cx: &App) -> impl IntoElement {
    div()
        .w_full()
        .text_color(cx.theme().danger)
        .child(reason.to_string())
}

/// 一行只读项：左侧标签，右侧取值。
pub fn value_row(label: &'static str, value: &str, cx: &App) -> AnyElement {
    let value = match value.is_empty() {
        true => EMPTY,
        false => value,
    };

    h_flex()
        .w_full()
        .items_start()
        .gap(COLUMN_GAP)
        .child(label_cell(label, cx))
        .child(
            // 取值是标识符，用等宽字体与正文区分；`flex_1` 配 `min_w_0` 才能在窄窗口里换行。
            div()
                .flex_1()
                .min_w_0()
                .font_family(cx.theme().mono_font_family.clone())
                .child(value.to_string()),
        )
        .into_any_element()
}

/// 一行可编辑项：左侧标签，右侧输入框，校验说明贴在被描述的输入框下方。
pub fn input_row(
    label: &'static str,
    state: &Entity<InputState>,
    error: Option<&'static str>,
    cx: &App,
) -> AnyElement {
    let hint = error.map(|text| {
        div()
            .text_color(cx.theme().danger)
            .child(text)
            .into_any_element()
    });

    h_flex()
        .w_full()
        .items_start()
        .gap(COLUMN_GAP)
        .child(label_cell(label, cx))
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap_1()
                .child(Input::new(state).cleanable(true).aria_label(label))
                .children(hint),
        )
        .into_any_element()
}

/// 一行可编辑的下拉项：左侧标签，右侧下拉框，说明贴在下方。
pub fn select_row(
    label: &'static str,
    select: AnyElement,
    hint: Option<&'static str>,
    cx: &App,
) -> AnyElement {
    let hint = hint.map(|text| {
        div()
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(text)
            .into_any_element()
    });

    h_flex()
        .w_full()
        .items_start()
        .gap(COLUMN_GAP)
        .child(label_cell(label, cx))
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap_1()
                .child(select)
                .children(hint),
        )
        .into_any_element()
}

/// 行首的标签单元格。
fn label_cell(label: &'static str, cx: &App) -> AnyElement {
    div()
        .w(LABEL_WIDTH)
        .flex_shrink_0()
        .text_color(cx.theme().muted_foreground)
        .child(label)
        .into_any_element()
}
