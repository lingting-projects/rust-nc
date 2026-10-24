use gpui_kit::base::{StyledExt as _, h_flex, v_flex};
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

use lingting_gpui_kit::card::Card;

use super::Console;
use super::LoadState;
use super::PADDING;
use crate::components::PolicyTags;
use crate::utils::format;

/// 渲染预览栏。
///
/// 一行元信息：配置标签、无区域节点规则、规则数量、剩余流量、有效期、上次更新。
/// 没有选中配置时整行不渲染。
pub fn render(console: &Console, cx: &App) -> AnyElement {
    let theme = cx.theme();

    // 数据未就绪时只保留加载提示，避免出现半截元信息。
    let data = match &console.load {
        LoadState::Ready(data) => data,
        LoadState::Loading => {
            return hint("正在加载配置…", theme.muted_foreground);
        }
        LoadState::Failed(reason) => {
            return hint(reason, theme.danger);
        }
    };

    let Some(config) = console.selected_config() else {
        return hint("暂无配置，请先在配置页新增", theme.muted_foreground);
    };

    let mut items: Vec<AnyElement> = Vec::new();

    // 节点策略与配置表共用同一组件，两处的取值、配色与说明不会走偏。
    items.push(PolicyTags::render(config, rems(0.5)).into_any_element());

    let rules =
        config.rule_direct_ids.len() + config.rule_proxy_ids.len() + config.rule_reject_ids.len();
    items.push(pair("规则数量", &rules.to_string(), theme).into_any_element());

    if let Some(subscribe) = config
        .subscribe_id
        .and_then(|id| data.subscribes.iter().find(|subscribe| subscribe.id == id))
    {
        items.push(
            pair(
                "剩余流量",
                &format::remaining(subscribe.max, subscribe.upload + subscribe.download),
                theme,
            )
            .into_any_element(),
        );
        items.push(pair("有效期", &format::until(subscribe.expire_time), theme).into_any_element());
        items.push(
            pair("上次更新", &format::since(subscribe.refresh_time), theme).into_any_element(),
        );
    }

    bar(h_flex().w_full().flex_wrap().gap_4().children(items))
}

/// 预览栏的卡片表面。
///
/// 内部按内容高度排布，不随内容区剩余高度拉伸。
fn bar(content: impl IntoElement) -> AnyElement {
    Card::new()
        .flex_shrink_0()
        .content_padding(PADDING)
        .child(content)
        .into_any_element()
}

/// 无法展示元信息时的一行提示。
///
/// 与正常状态共用同一张卡片，状态切换时布局不跳动。
fn hint(reason: &str, color: Hsla) -> AnyElement {
    bar(div().w_full().text_color(color).child(reason.to_string()))
}

/// 「标签 + 数值」的一项元信息。
fn pair(label: &'static str, value: &str, theme: &gpui_kit::component::Theme) -> impl IntoElement {
    h_flex()
        .gap_2()
        .child(div().text_color(theme.muted_foreground).child(label))
        .child(
            div()
                .font_weight(FontWeight::MEDIUM)
                .child(value.to_string()),
        )
}
