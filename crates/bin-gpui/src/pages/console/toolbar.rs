//! 控制栏。
//!
//! 从左到右：启动/停止按钮、刷新按钮、配置列表下拉框。

use gpui_kit::base::{StyledExt as _, h_flex};
use gpui_kit::component::IndexPath;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants as _;
use gpui_kit::component::searchable_list::SearchableListItem;
use gpui_kit::component::select::{SearchableVec, Select, SelectEvent, SelectState};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, IconName, Sizable as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use lingting_gpui_kit::card::Card;

use super::Console;
use super::LoadState;
use super::PADDING;

/// 配置下拉框的宽度。
const SELECT_WIDTH: Rems = rems(20.);

/// 渲染控制栏。
pub fn render(console: &Console, cx: &mut Context<Console>) -> impl IntoElement {
    let ready = matches!(console.load, LoadState::Ready(_));
    let running = console.kernel.running;
    let has_config = console.selected.is_some();

    // 启动/停止按钮：停止态是启动，运行态是停止。
    let action = Button::new("kernel-toggle")
        .icon(if running {
            IconName::Square
        } else {
            IconName::Play
        })
        .label(if running { "停止" } else { "启动" })
        .loading(console.kernel.loading)
        .disabled(!ready || !has_config)
        .when(running, |this| this.danger())
        .when(!running, |this| this.primary())
        .on_click(cx.listener(|console, _, _, cx| console.toggle_kernel(cx)));

    Card::new().flex_shrink_0().content_padding(PADDING).child(
        h_flex()
            .w_full()
            .gap_3()
            .child(action)
            .child(
                Button::new("config-refresh")
                    .icon(IconName::RefreshCw)
                    .label("刷新")
                    .loading(console.refreshing)
                    .on_click(cx.listener(|console, _, window, cx| console.reload(window, cx))),
            )
            .child(
                // 内核运行时禁用，避免切换正在生效的配置。
                div().w(SELECT_WIDTH).child(
                    Select::new(&console.select)
                        .id("config-select")
                        .disabled(running)
                        .placeholder("选择配置")
                        .accessibility_label("配置列表"),
                ),
            ),
    )
}

/// 配置下拉框的一个选项：`value` 是配置 id，展示名称。
#[derive(Clone)]
pub struct ConfigOption {
    id: SharedString,
    name: SharedString,
}

impl ConfigOption {
    pub fn new(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
        }
    }
}

impl SearchableListItem for ConfigOption {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.name.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.id
    }
}

/// 创建配置下拉框的状态。
///
/// 状态由页面持有：每次刷新后调用 [`replace`] 换掉选项，
/// 选择变化通过 [`SelectEvent`] 回到页面。
pub fn create_state(
    window: &mut Window,
    cx: &mut Context<Console>,
) -> Entity<SelectState<SearchableVec<ConfigOption>>> {
    cx.new(|cx| SelectState::new(SearchableVec::new(Vec::new()), None, window, cx))
}

/// 换掉下拉框的选项并选中指定配置。
pub fn replace(
    state: &Entity<SelectState<SearchableVec<ConfigOption>>>,
    options: Vec<ConfigOption>,
    selected: Option<&str>,
    window: &mut Window,
    cx: &mut App,
) {
    let index = selected.and_then(|selected| {
        options
            .iter()
            .position(|option| option.id.as_ref() == selected)
            .map(IndexPath::new)
    });

    state.update(cx, |state, cx| {
        state.set_items(SearchableVec::new(options), window, cx);
        state.set_selected_index(index, window, cx);
    });
}

/// 把下拉框的选择变化接到页面状态上。
pub fn subscribe(
    state: &Entity<SelectState<SearchableVec<ConfigOption>>>,
    cx: &mut Context<Console>,
) -> Subscription {
    cx.subscribe(
        state,
        |console, _, event: &SelectEvent<SearchableVec<ConfigOption>>, cx| {
            let SelectEvent::Confirm(Some(value)) = event else {
                return;
            };
            if console.selected.as_deref() != Some(value.as_ref()) {
                console.selected = Some(value.to_string());
                cx.notify();
            }
        },
    )
}
