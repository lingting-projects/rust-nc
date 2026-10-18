use gpui_kit::base::{StyledExt as _, h_flex};
use gpui_kit::component::tag::Tag;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{ColorName, Sizable as _};
use gpui_kit::*;

use types_core::entity::config::Config;

/// 节点策略标签组。
///
/// 配置表与配置预览需要展示同一组「节点策略」，两边各自拼标签会慢慢走偏，
/// 因此把取值、配色与 tooltip 说明集中在这里：调用方只给配置与间距，
/// 不再关心有哪几项、什么颜色、怎么描述。
///
/// 每一项用自己的颜色，扫读时不必逐个读文字；无区域节点的颜色同时表示
/// 包含还是排除，其余标签只表示「启用了这一项」，关闭时不渲染。
pub struct PolicyTags;

impl PolicyTags {
    /// 渲染标签组。
    pub fn render(config: &Config, gap: Rems) -> impl IntoElement {
        h_flex().gap(gap).children(tags(config))
    }
}

/// 各策略标签。
fn tags(config: &Config) -> Vec<AnyElement> {
    let mut tags = vec![if config.include_area_non {
        tag(
            Tag::info(),
            "包含无区域节点",
            "筛选时保留没有区域信息的节点",
        )
    } else {
        tag(
            Tag::warning(),
            "排除无区域节点",
            "筛选时移除没有区域信息的节点",
        )
    }];

    if config.tun {
        tags.push(tag(
            Tag::color(ColorName::Violet),
            "TUN",
            "使用 TUN 模式接管流量",
        ));
    }
    tags.push(if config.ipv6 {
        tag(Tag::color(ColorName::Emerald), "IPv6", "启用 IPv6")
    } else {
        tag(Tag::color(ColorName::Cyan), "IPv4", "仅使用 IPv4")
    });
    if config.fake_ip {
        tags.push(tag(
            Tag::color(ColorName::Fuchsia),
            "FakeIP",
            "启用 FakeIP",
        ));
    }

    tags
}

/// 一枚带 tooltip 的策略标签：颜色区分类型，tooltip 说明这一项的含义。
///
/// `Tag` 自身不接受交互与 tooltip，因此外面包一层 `div` 承载 tooltip 与可读名称。
fn tag(tag: Tag, label: &'static str, hint: &'static str) -> AnyElement {
    div()
        .id(label)
        .aria_label(label)
        .tooltip(move |window, cx| Tooltip::new(hint).build(window, cx))
        .child(tag.small().child(label))
        .into_any_element()
}
