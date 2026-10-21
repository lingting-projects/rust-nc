use gpui_kit::assets::IconName;
use gpui_kit::base::{StyledExt as _, h_flex};
use gpui_kit::component::Disableable as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants as _;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::*;

use lingting_gpui_kit::card::Card;

use super::ConnectionPage;
use super::PADDING;
use super::filter;

/// 渲染搜索栏。
///
/// 从左到右：状态筛选、暂停/继续、清空、搜索输入框。
/// 输入框占满除按钮以外的全部宽度，输入内容即时过滤表格。
pub fn render(page: &ConnectionPage, cx: &mut Context<ConnectionPage>) -> impl IntoElement {
    let query = page.query();
    let paused = page.is_paused();

    Card::new().flex_shrink_0().content_padding(PADDING).child(
        h_flex()
            .w_full()
            .flex_nowrap()
            .gap_3()
            .child(filter::render(page, cx))
            .child(
                // 暂停只影响轮询，已读到的数据保持展示，按钮文案说明再次点击的结果。
                Button::new("connection-pause")
                    .icon(if paused {
                        IconName::CirclePlay
                    } else {
                        IconName::CirclePause
                    })
                    .label(if paused { "继续" } else { "暂停" })
                    .on_click(cx.listener(|page, _, _, cx| page.toggle_pause(cx))),
            )
            .child(
                Button::new("connection-clear")
                    .icon(IconName::Eraser)
                    .label("清空")
                    .disabled(page.is_empty())
                    .on_click(cx.listener(|page, _, _, cx| page.clear(cx))),
            )
            .child(
                // 输入框占据剩余宽度：`flex_1` 配 `min_w_0` 才能在窄窗口里被压缩。
                div()
                    .flex_1()
                    .min_w_0()
                    .child(Input::new(&query).cleanable(true).aria_label("连接搜索")),
            ),
    )
}

/// 创建搜索框的状态。
pub fn create_state(window: &mut Window, cx: &mut Context<ConnectionPage>) -> Entity<InputState> {
    cx.new(|cx| InputState::new(window, cx).placeholder("搜索入口、进程、目标、规则或源地址"))
}

/// 把搜索框的输入变化接到页面的关键字上。
pub fn subscribe(state: &Entity<InputState>, cx: &mut Context<ConnectionPage>) -> Subscription {
    cx.subscribe(state, |page, state, event: &InputEvent, cx| {
        if matches!(event, InputEvent::Change) {
            let keyword = state.read(cx).value().to_string();
            page.set_keyword(keyword, cx);
        }
    })
}
