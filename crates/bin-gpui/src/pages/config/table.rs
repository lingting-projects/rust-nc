use std::collections::HashSet;
use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::base::{StyledExt as _, h_flex};
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants as _;
use gpui_kit::component::table::{Column, DataTable, TableDelegate, TableState};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{ActiveTheme as _, Sizable as _};
use gpui_kit::*;

use lingting_gpui_kit::region::RegionTag;

use types_core::entity::config::Config;

use super::ConfigPage;
use crate::components::{PolicyTags, delete_button};
use crate::utils::format;
use crate::utils::refresh::RefreshState;

/// 各列宽度，单位为 rem，随界面缩放一起变化。
const NAME_WIDTH: Rems = rems(10.);
const COUNT_WIDTH: Rems = rems(5.);
const POLICY_WIDTH: Rems = rems(14.);
const STATE_WIDTH: Rems = rems(7.);
const REFRESH_WIDTH: Rems = rems(7.);
const ACTION_WIDTH: Rems = rems(9.);

/// 列定义：键、表头文案、宽度。
const COLUMNS: [(&str, &str, Rems); 6] = [
    ("name", "名称", NAME_WIDTH),
    ("count", "规则数量", COUNT_WIDTH),
    ("policy", "节点策略", POLICY_WIDTH),
    ("state", "状态", STATE_WIDTH),
    ("refresh", "上次更新", REFRESH_WIDTH),
    ("action", "操作", ACTION_WIDTH),
];

/// 建立表格状态。
pub fn create(
    window: &mut Window,
    cx: &mut Context<ConfigPage>,
) -> Entity<TableState<ConfigTable>> {
    let page = cx.entity().downgrade();
    let rem_size = window.rem_size();

    cx.new(|cx| {
        TableState::new(ConfigTable::new(page, rem_size), window, cx)
            .col_movable(false)
            .col_resizable(false)
            .sortable(false)
            .row_selectable(false)
            .col_selectable(false)
    })
}

/// 把页面的数据、过滤结果与当前缩放同步给表格。
///
/// 每帧调用，内容因此总是与页面一致；只有缩放变化时才重建列宽，
/// 因为列宽由 rem 换算而来，缩放后必须重算，否则表头与单元格会错位。
pub fn sync(
    table: &Entity<TableState<ConfigTable>>,
    items: Rc<Vec<Config>>,
    rows: Rc<Vec<usize>>,
    pending: Rc<HashSet<String>>,
    rem_size: Pixels,
    cx: &mut App,
) {
    table.update(cx, |state, cx| {
        let changed = state.delegate_mut().set_content(items, rows);
        let refreshing = state.delegate_mut().set_pending(pending);
        let resized = state.delegate_mut().set_rem_size(rem_size);
        // 委托只持有数据，表格实体本身不会被通知：不主动通知的话，
        // 保留渲染会复用上一帧，列表就停在旧内容上。
        if changed || refreshing || resized {
            state.refresh(cx);
            cx.notify();
        }
    });
}

/// 渲染表格。
pub fn render(table: &Entity<TableState<ConfigTable>>) -> AnyElement {
    DataTable::new(table)
        .bordered(false)
        .small()
        .into_any_element()
}

/// 加载中与失败时共用的占位，与表格占同一块区域，状态切换时布局不跳动。
pub fn placeholder(reason: &str, cx: &App) -> AnyElement {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .text_color(cx.theme().muted_foreground)
        .child(reason.to_string())
        .into_any_element()
}

/// 关键字命中的下标。
///
/// 配置只按名称做包含匹配，与参考实现的筛选口径一致。
pub fn matched(items: &[Config], keyword: &str) -> Vec<usize> {
    let keyword = keyword.trim().to_lowercase();
    if keyword.is_empty() {
        return (0..items.len()).collect();
    }

    (0..items.len())
        .filter(|ix| items[*ix].name.to_lowercase().contains(&keyword))
        .collect()
}

/// 配置表格的数据与渲染。
///
/// 六列：名称、规则数量、节点策略、状态、上次更新、操作。
/// 委托只持有数据与过滤结果的共享引用，由页面在渲染时同步；
/// 页面是数据的唯一来源，表格不单独维护一份可变副本。
pub struct ConfigTable {
    items: Rc<Vec<Config>>,
    rows: Rc<Vec<usize>>,
    /// 正在刷新中的配置 id，来自同步调度器
    pending: Rc<HashSet<String>>,
    rem_size: Pixels,
    page: WeakEntity<ConfigPage>,
}

impl ConfigTable {
    fn new(page: WeakEntity<ConfigPage>, rem_size: Pixels) -> Self {
        Self {
            items: Rc::new(Vec::new()),
            rows: Rc::new(Vec::new()),
            pending: Rc::new(HashSet::new()),
            rem_size,
            page,
        }
    }

    /// 更新数据与过滤结果，返回是否发生了变化。
    fn set_content(&mut self, items: Rc<Vec<Config>>, rows: Rc<Vec<usize>>) -> bool {
        let changed = !Rc::ptr_eq(&self.items, &items) || !Rc::ptr_eq(&self.rows, &rows);
        self.items = items;
        self.rows = rows;
        changed
    }

    /// 更新刷新中的配置集合，返回是否发生了变化。
    fn set_pending(&mut self, pending: Rc<HashSet<String>>) -> bool {
        let changed = !Rc::ptr_eq(&self.pending, &pending);
        self.pending = pending;
        changed
    }

    /// 更新缩放基准，返回是否发生了变化。
    fn set_rem_size(&mut self, rem_size: Pixels) -> bool {
        if self.rem_size == rem_size {
            return false;
        }
        self.rem_size = rem_size;
        true
    }

    /// 第 `row_ix` 行对应的配置，行号是过滤后的位置。
    fn item(&self, row_ix: usize) -> Option<&Config> {
        self.rows.get(row_ix).map(|ix| &self.items[*ix])
    }

    /// 名称列：只放名称，其余信息各自成列，避免同一行重复表达。
    ///
    /// 名称长度不可控，超出列宽时省略，完整名称放在 tooltip 里。
    fn name(&self, item: &Config) -> AnyElement {
        let name = item.name.clone();
        div()
            .font_weight(FontWeight::MEDIUM)
            .truncate()
            .id(ElementId::from(format!("config-name:{}", item.id)))
            .aria_label(name.clone())
            .tooltip(move |window, cx| Tooltip::new(name.clone()).build(window, cx))
            .child(item.name.clone())
            .into_any_element()
    }

    /// 状态列：未刷新、刷新中、已刷新。
    fn state(&self, item: &Config) -> AnyElement {
        let refreshing = self.pending.contains(&item.id);
        RefreshState::of(item.refresh_time, refreshing).render()
    }

    /// 操作列：同步、编辑、删除。
    ///
    /// 三个都是图标按钮，用 tooltip 说明含义，并以 `accessibility_label` 给出可读名称。
    fn action(&self, item: &Config, cx: &App) -> AnyElement {
        let page = self.page.clone();
        let id = item.id.clone();
        let name = item.name.clone();
        // 按钮标识跟随配置 id：过滤后行号会变，用行号会串掉按钮的内部状态。
        let sync_id: ElementId = format!("config-sync:{}", item.id).into();
        let edit_id: ElementId = format!("config-edit:{}", item.id).into();
        let delete_id: ElementId = format!("config-delete:{}", item.id).into();

        h_flex()
            .gap_1()
            .child(
                Button::new(sync_id)
                    .ghost()
                    .small()
                    .icon(IconName::RefreshCw)
                    .tooltip("同步")
                    .accessibility_label("同步")
                    .on_click({
                        let page = page.clone();
                        let id = id.clone();
                        move |_, window, cx| {
                            page.update(cx, |page, cx| page.sync_one(&id, window, cx))
                                .ok();
                        }
                    }),
            )
            .child(
                Button::new(edit_id)
                    .ghost()
                    .small()
                    .icon(IconName::Pencil)
                    .tooltip("编辑")
                    .accessibility_label("编辑")
                    .on_click({
                        let page = page.clone();
                        let id = id.clone();
                        move |_, window, cx| {
                            page.update(cx, |page, cx| page.edit(&id, window, cx)).ok();
                        }
                    }),
            )
            .child(
                delete_button(delete_id, IconName::Trash, "删除", cx).on_click(
                    move |_, window, cx| {
                        super::dialog::confirm_delete(
                            page.clone(),
                            id.clone(),
                            name.clone(),
                            window,
                            cx,
                        );
                    },
                ),
            )
            .into_any_element()
    }
}

impl TableDelegate for ConfigTable {
    fn columns_count(&self, _: &App) -> usize {
        COLUMNS.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        self.rows.len()
    }

    fn column(&self, col_ix: usize, _: &App) -> Column {
        let (key, name, width) = COLUMNS[col_ix];
        Column::new(key, name)
            .width(width.to_pixels(self.rem_size))
            .resizable(false)
            .movable(false)
            .selectable(false)
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let Some(item) = self.item(row_ix) else {
            return div().into_any_element();
        };

        match col_ix {
            0 => self.name(item),
            1 => count(item).into_any_element(),
            2 => policy(item).into_any_element(),
            3 => self.state(item),
            4 => div()
                .child(format::since(item.refresh_time))
                .into_any_element(),
            _ => self.action(item, cx),
        }
    }
}

/// 规则数量：直连、代理、拒绝三类规则的数量之和。
fn count(item: &Config) -> impl IntoElement {
    let total = item.rule_direct_ids.len() + item.rule_proxy_ids.len() + item.rule_reject_ids.len();
    div().child(total.to_string())
}

/// 节点策略：把各列原本单独展示的开关收成一排标签。
///
/// 取值、配色与说明由 [`PolicyTags`] 统一维护，与配置预览栏共用同一实现。
fn policy(item: &Config) -> impl IntoElement {
    PolicyTags::render(item, rems(0.25))
}
