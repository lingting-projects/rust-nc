use std::rc::Rc;

use gpui_kit::base::{StyledExt as _, h_flex, v_flex};
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::*;

use lingting_gpui_kit::card::{Card, ItemCard};
use lingting_gpui_kit::virtual_card::VirtualCard;
use lingting_gpui_kit::virtual_list::ItemFilter;

use super::NodePage;
use super::PADDING;
use super::delay::Delay;
use crate::services::GroupNode;

/// 节点行高度。
const ITEM_HEIGHT: Rems = rems(2.75);

/// 渲染右栏：节点搜索框与选中分组的节点列表。
pub fn render(page: &NodePage, cx: &mut Context<NodePage>) -> AnyElement {
    let keyword = page.node_keyword().to_string();
    let items = Rc::new(page.nodes().map(<[GroupNode]>::to_vec).unwrap_or_default());
    let filter: ItemFilter<GroupNode> = Rc::new(move |node| matched(node, &keyword));
    let selected = page.current().map(str::to_string);
    // 只有手动选择分组才能切换节点，自动选择分组的节点行只读。
    let switchable = page.switchable();

    let list = VirtualCard::new("node-items")
        .with_ghost()
        .items(items)
        .item_height(ITEM_HEIGHT)
        .filter(filter)
        .item_renderer(Rc::new({
            let entity = cx.entity();
            move |node, _, cx| render_item(&entity, node, selected.as_deref(), switchable, cx)
        }))
        .empty(empty(page, cx))
        .into_any_element();

    v_flex()
        .flex_1()
        .min_w_0()
        .h_full()
        .min_h_0()
        .gap_2()
        .child(
            Card::new().flex_shrink_0().content_padding(PADDING).child(
                Input::new(&page.node_query())
                    .cleanable(true)
                    .aria_label("节点搜索"),
            ),
        )
        .child(div().flex_1().min_h_0().child(list))
        .into_any_element()
}

/// 节点行的中间栏：节点名、类型与延迟。
fn render_item(
    entity: &Entity<NodePage>,
    node: &GroupNode,
    selected: Option<&str>,
    switchable: bool,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    let is_selected = selected == Some(node.tag.as_str());

    let center = h_flex()
        .w_full()
        .gap_2()
        .child(div().flex_1().min_w_0().truncate().child(node.tag.clone()))
        .child(
            div()
                .flex_shrink_0()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(node.kind.clone()),
        )
        .child(Delay::of(node.delay).render(theme));

    let card = ItemCard::new(format!("node-item:{}", node.tag))
        .selected(is_selected)
        .hoverable(switchable)
        .center(center);

    if !switchable {
        return card.into_any_element();
    }

    let entity = entity.clone();
    let tag = node.tag.clone();
    card.on_click(move |_, cx| {
        entity.update(cx, |page, cx| page.switch_node(&tag, cx));
    })
    .into_any_element()
}

/// 空状态：未选中分组、分组内没有节点或搜索无命中。
fn empty(page: &NodePage, cx: &App) -> AnyElement {
    let text = if page.selected().is_none() {
        "请选择左侧分组"
    } else if page.node_keyword().trim().is_empty() {
        "该分组没有节点"
    } else {
        "没有匹配的节点"
    };

    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(text)
        .into_any_element()
}

/// 节点是否命中关键字：按节点名与类型做包含匹配。
fn matched(node: &GroupNode, keyword: &str) -> bool {
    let keyword = keyword.trim().to_lowercase();
    if keyword.is_empty() {
        return true;
    }
    node.tag.to_lowercase().contains(&keyword) || node.kind.to_lowercase().contains(&keyword)
}

/// 创建节点搜索框的状态。
pub fn create_state(window: &mut Window, cx: &mut Context<NodePage>) -> Entity<InputState> {
    cx.new(|cx| InputState::new(window, cx).placeholder("搜索节点"))
}

/// 把节点搜索框的输入变化接到页面状态上。
pub fn subscribe(state: &Entity<InputState>, cx: &mut Context<NodePage>) -> Subscription {
    cx.subscribe(state, |page, state, event: &InputEvent, cx| {
        if matches!(event, InputEvent::Change) {
            let keyword = state.read(cx).value().to_string();
            page.set_node_keyword(keyword, cx);
        }
    })
}
