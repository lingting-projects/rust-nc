use gpui_kit::assets::IconName;
use gpui_kit::base::{StyledExt as _, h_flex};
use gpui_kit::component::Sizable as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants as _;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::*;

use lingting_gpui_kit::card::Card;

use super::ConfigPage;
use super::PADDING;

/// 渲染搜索栏。
///
/// 从左到右：同步按钮、新增按钮、搜索输入框、查询按钮。
/// 输入框占满除按钮以外的全部宽度，输入内容即时过滤表格。
pub fn render(page: &ConfigPage, cx: &mut Context<ConfigPage>) -> impl IntoElement {
    let query = page.query();

    Card::new().flex_shrink_0().content_padding(PADDING).child(
        h_flex()
            .w_full()
            .gap_3()
            .child(
                Button::new("config-sync")
                    .icon(IconName::RefreshCw)
                    .label("同步")
                    .loading(page.is_syncing())
                    .on_click(cx.listener(|page, _, window, cx| page.sync_all(window, cx))),
            )
            .child(
                Button::new("config-create")
                    .icon(IconName::Plus)
                    .label("新增")
                    .on_click(cx.listener(|page, _, window, cx| page.create(window, cx))),
            )
            .child(
                // 输入框占据剩余宽度：`flex_1` 配 `min_w_0` 才能在窄窗口里被压缩。
                div()
                    .flex_1()
                    .min_w_0()
                    .child(Input::new(&query).cleanable(true).aria_label("配置搜索")),
            )
            .child(
                Button::new("config-query")
                    .icon(IconName::Search)
                    .label("查询")
                    .on_click(cx.listener(|page, _, _, cx| page.reload(cx))),
            ),
    )
}

/// 创建搜索输入框的状态。
pub fn create_state(window: &mut Window, cx: &mut Context<ConfigPage>) -> Entity<InputState> {
    cx.new(|cx| InputState::new(window, cx).placeholder("输入名称筛选"))
}

/// 把输入变化接到页面的搜索关键字上。
///
/// 输入即过滤，因此查询按钮只承担「重新读取数据」的职责。
pub fn subscribe(state: &Entity<InputState>, cx: &mut Context<ConfigPage>) -> Subscription {
    cx.subscribe(state, |page, state, event: &InputEvent, cx| {
        if matches!(event, InputEvent::Change) {
            let keyword = state.read(cx).value().to_string();
            page.set_keyword(keyword, cx);
        }
    })
}
