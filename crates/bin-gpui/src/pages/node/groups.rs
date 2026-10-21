use std::collections::HashSet;
use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::base::{StyledExt as _, h_flex, v_flex};
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::Sizable as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants as _;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::*;

use lingting_gpui_kit::card::{Card, ItemCard};
use lingting_gpui_kit::virtual_card::VirtualCard;
use lingting_gpui_kit::virtual_list::ItemFilter;

use super::NodePage;
use super::PADDING;
use super::delay::Delay;
use crate::services::GroupInfo;

/// 左栏宽度。右栏吃掉剩余宽度，因此左栏固定宽度，列表滚动区不会随分组名长短跳动。
pub const WIDTH: Rems = rems(20.);

/// 分组行高度。
const ITEM_HEIGHT: Rems = rems(4.5);

/// 渲染左栏：标题与刷新按钮、分组搜索框、分组列表。
pub fn render(page: &NodePage, cx: &mut Context<NodePage>) -> AnyElement {
    let list = match page.groups() {
        Some(groups) => {
            let keyword = page.group_keyword().to_string();
            let filter: ItemFilter<GroupInfo> = Rc::new(move |group| matched(group, &keyword));
            let selected = page.selected().map(str::to_string);
            // 测速中的分组在这里先取快照：行渲染时页面实体正被渲染，不能再回读。
            let testing = page.testing_snapshot();

            VirtualCard::new("node-groups")
                .with_ghost()
                .items(Rc::new(groups.to_vec()))
                .item_height(ITEM_HEIGHT)
                .filter(filter)
                .item_renderer(Rc::new({
                    let entity = cx.entity();
                    move |group, _, cx| {
                        render_item(&entity, group, selected.as_deref(), &testing, cx)
                    }
                }))
                .empty(empty(cx))
                .into_any_element()
        }
        None => placeholder(page, cx),
    };

    v_flex()
        .w(WIDTH)
        .flex_shrink_0()
        .h_full()
        .min_h_0()
        .gap_2()
        .child(
            Card::new().flex_shrink_0().content_padding(PADDING).child(
                v_flex()
                    .w_full()
                    .gap_2()
                    .child(
                        h_flex()
                            .w_full()
                            .gap_2()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child("分组"),
                            )
                            .child(
                                Button::new("node-group-refresh")
                                    .ghost()
                                    .small()
                                    .icon(IconName::RefreshCw)
                                    .loading(page.is_refreshing())
                                    .tooltip("刷新")
                                    .accessibility_label("刷新分组")
                                    .on_click(
                                        cx.listener(|page, _, window, cx| page.reload(window, cx)),
                                    ),
                            ),
                    )
                    .child(
                        Input::new(&page.group_query())
                            .cleanable(true)
                            .aria_label("分组搜索"),
                    ),
            ),
        )
        .child(div().flex_1().min_h_0().child(list))
        .into_any_element()
}

/// 分组行的中间栏：上为名称与节点统计，下为选中方式与当前节点。
fn render_item(
    entity: &Entity<NodePage>,
    group: &GroupInfo,
    selected: Option<&str>,
    testing: &HashSet<String>,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    let is_selected = selected == Some(group.tag.as_str());
    let stats = Stats::of(group);

    let top = h_flex()
        .w_full()
        .gap_2()
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .font_weight(FontWeight::MEDIUM)
                .child(group.tag.clone()),
        )
        .child(count(stats.normal, "正常", theme.success))
        .child(count(stats.slow, "慢", theme.warning))
        .child(count(stats.total, "总", theme.muted_foreground));

    let bottom = h_flex()
        .w_full()
        .gap_2()
        .child(mode_label(group.selectable, theme))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .child(group.selected.clone()),
        )
        .child(Delay::of(group.delay()).render(theme));

    let center = v_flex()
        .w_full()
        .h_full()
        .justify_center()
        .gap_1()
        // 上半略高于下半：名称与统计是主信息，选中摘要读作次要信息
        .child(div().flex_grow(1.2).min_h_0().child(top))
        .child(div().flex_grow(1.).min_h_0().child(bottom));

    let loading = testing.contains(&group.tag);
    let action = Button::new(format!("node-group-test:{}", group.tag))
        .ghost()
        .small()
        .icon(IconName::Zap)
        .loading(loading)
        .tooltip("测速")
        .accessibility_label("测速")
        .on_click({
            let entity = entity.clone();
            let tag = group.tag.clone();
            move |_, window, cx| {
                entity.update(cx, |page, cx| page.url_test(&tag, window, cx));
            }
        });

    div()
        .w_full()
        .h_full()
        .pb_1()
        .child(
            ItemCard::new(format!("node-group:{}", group.tag))
                .h_full()
                .selected(is_selected)
                .center(center)
                .right(action)
                .on_click({
                    let entity = entity.clone();
                    let tag = group.tag.clone();
                    move |_, cx| {
                        entity.update(cx, |page, cx| page.select_group(&tag, cx));
                    }
                }),
        )
        .into_any_element()
}

/// 一组节点统计。
struct Stats {
    normal: usize,
    slow: usize,
    total: usize,
}

impl Stats {
    fn of(group: &GroupInfo) -> Self {
        let mut stats = Self {
            normal: 0,
            slow: 0,
            total: group.items.len(),
        };
        for item in &group.items {
            match Delay::of(item.delay) {
                Delay::Normal(_) => stats.normal += 1,
                Delay::Slow(_) => stats.slow += 1,
                Delay::Unknown => {}
            }
        }
        stats
    }
}

/// 一项节点统计，颜色即该档次的颜色。
fn count(value: usize, label: &'static str, color: Hsla) -> impl IntoElement {
    h_flex()
        .flex_shrink_0()
        .gap_1()
        .text_xs()
        .text_color(color)
        .child(value.to_string())
        .child(label)
}

/// 分组的切换方式：可手动切换是「手动选择」，否则是自动选择。
fn mode_label(selectable: bool, theme: &gpui_kit::component::Theme) -> impl IntoElement {
    let (text, color) = if selectable {
        ("手动选择", theme.primary)
    } else {
        ("自动选择", theme.muted_foreground)
    };

    div()
        .flex_shrink_0()
        .text_xs()
        .text_color(color)
        .child(text)
}

/// 空状态：分组搜索没有命中。
fn empty(cx: &App) -> AnyElement {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child("没有匹配的分组")
        .into_any_element()
}

/// 加载中与失败时的占位，与列表占同一块区域，状态切换时布局不跳动。
fn placeholder(page: &NodePage, cx: &App) -> AnyElement {
    let (text, color) = match page.failure() {
        Some(reason) => (reason.to_string(), cx.theme().danger),
        None => ("正在加载分组…".to_string(), cx.theme().muted_foreground),
    };

    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(color)
        .child(text)
        .into_any_element()
}

/// 分组是否命中关键字：按分组名与当前选中节点名做包含匹配。
fn matched(group: &GroupInfo, keyword: &str) -> bool {
    let keyword = keyword.trim().to_lowercase();
    if keyword.is_empty() {
        return true;
    }
    group.tag.to_lowercase().contains(&keyword) || group.selected.to_lowercase().contains(&keyword)
}

/// 创建分组搜索框的状态。
pub fn create_state(window: &mut Window, cx: &mut Context<NodePage>) -> Entity<InputState> {
    cx.new(|cx| InputState::new(window, cx).placeholder("搜索分组"))
}

/// 把分组搜索框的输入变化接到页面状态上。
pub fn subscribe(state: &Entity<InputState>, cx: &mut Context<NodePage>) -> Subscription {
    cx.subscribe(state, |page, state, event: &InputEvent, cx| {
        if matches!(event, InputEvent::Change) {
            let keyword = state.read(cx).value().to_string();
            page.set_group_keyword(keyword, cx);
        }
    })
}
