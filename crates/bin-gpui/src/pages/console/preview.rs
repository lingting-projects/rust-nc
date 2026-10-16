//! 预览栏。
//!
//! 一行元信息：配置标签、无区域节点规则、规则数量、剩余流量、有效期、上次更新。
//! 没有选中配置时整行不渲染。

use gpui_kit::base::{StyledExt as _, h_flex, v_flex};
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme as _, Sizable as _};
use gpui_kit::*;

use super::Console;
use super::LoadState;
use super::format;

/// 渲染预览栏。
pub fn render(console: &Console, cx: &App) -> AnyElement {
    let theme = cx.theme();

    // 数据未就绪时只保留加载提示，避免出现半截元信息。
    let data = match &console.load {
        LoadState::Ready(data) => data,
        LoadState::Loading => {
            return v_flex()
                .w_full()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child("正在加载配置…")
                .into_any_element();
        }
        LoadState::Failed(reason) => {
            return v_flex()
                .w_full()
                .text_sm()
                .text_color(theme.danger)
                .child(reason.clone())
                .into_any_element();
        }
    };

    let Some(config) = console.selected_config() else {
        return v_flex()
            .w_full()
            .text_sm()
            .text_color(theme.muted_foreground)
            .child("暂无配置，请先在配置页新增")
            .into_any_element();
    };

    let mut items: Vec<AnyElement> = Vec::new();

    // 启用中的配置才展示协议标签。
    if config.tun {
        items.push(tag("TUN").into_any_element());
    }
    items.push(tag(if config.ipv6 { "IPv6" } else { "IPv4" }).into_any_element());
    if config.fake_ip {
        items.push(tag("FakeIP").into_any_element());
    }

    let rules =
        config.rule_direct_ids.len() + config.rule_proxy_ids.len() + config.rule_reject_ids.len();
    items.push(
        pair(
            "无区域节点",
            if config.include_area_non {
                "包含"
            } else {
                "排除"
            },
            theme,
        )
        .into_any_element(),
    );
    items.push(pair("规则数量", &rules.to_string(), theme).into_any_element());

    if let Some(subscribe) = data
        .subscribes
        .iter()
        .find(|subscribe| subscribe.id == config.subscribe_id)
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

    h_flex()
        .w_full()
        .flex_wrap()
        .gap_4()
        .children(items)
        .into_any_element()
}

/// 一枚中性的配置标签。
fn tag(label: &'static str) -> impl IntoElement {
    Tag::secondary().small().child(label)
}

/// 「标签 + 数值」的一项元信息。
fn pair(label: &'static str, value: &str, theme: &gpui_kit::component::Theme) -> impl IntoElement {
    h_flex()
        .gap_2()
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(label),
        )
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .child(value.to_string()),
        )
}
