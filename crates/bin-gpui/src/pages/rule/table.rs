use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::base::{StyledExt as _, h_flex, v_flex};
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants as _;
use gpui_kit::component::table::{Column, DataTable, TableDelegate, TableState};
use gpui_kit::component::{ActiveTheme as _, Sizable as _};
use gpui_kit::*;

use types_core::entity::rule::Rule;

use super::RulePage;
use crate::components::delete_button;
use crate::utils::format;

/// 各列宽度，单位为 rem，随界面缩放一起变化。
const NAME_WIDTH: Rems = rems(20.);
const COUNT_WIDTH: Rems = rems(8.);
const KIND_WIDTH: Rems = rems(6.);
const REFRESH_WIDTH: Rems = rems(8.);
const ACTION_WIDTH: Rems = rems(9.);

/// 列定义：键、表头文案、宽度。
const COLUMNS: [(&str, &str, Rems); 5] = [
    ("name", "名称", NAME_WIDTH),
    ("count", "规则数量", COUNT_WIDTH),
    ("kind", "类型", KIND_WIDTH),
    ("refresh", "上次更新", REFRESH_WIDTH),
    ("action", "操作", ACTION_WIDTH),
];

/// 建立表格状态。
pub fn create(window: &mut Window, cx: &mut Context<RulePage>) -> Entity<TableState<RuleTable>> {
    let page = cx.entity().downgrade();
    let rem_size = window.rem_size();

    cx.new(|cx| {
        TableState::new(RuleTable::new(page, rem_size), window, cx)
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
    table: &Entity<TableState<RuleTable>>,
    items: Rc<Vec<Rule>>,
    rows: Rc<Vec<usize>>,
    rem_size: Pixels,
    cx: &mut App,
) {
    table.update(cx, |state, cx| {
        state.delegate_mut().set_content(items, rows);
        if state.delegate_mut().set_rem_size(rem_size) {
            state.refresh(cx);
        }
    });
}

/// 渲染表格。
pub fn render(table: &Entity<TableState<RuleTable>>) -> AnyElement {
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
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(reason.to_string())
        .into_any_element()
}

/// 关键字命中的下标。
///
/// 规则只按名称与地址做包含匹配，与订阅的筛选口径一致。
pub fn matched(items: &[Rule], keyword: &str) -> Vec<usize> {
    let keyword = keyword.trim().to_lowercase();
    if keyword.is_empty() {
        return (0..items.len()).collect();
    }

    (0..items.len())
        .filter(|ix| {
            let item = &items[*ix];
            item.name.to_lowercase().contains(&keyword)
                || item.url.to_lowercase().contains(&keyword)
        })
        .collect()
}

/// 规则表格的数据与渲染。
///
/// 五列：名称、规则数量、类型、上次更新、操作。
/// 委托只持有数据与过滤结果的共享引用，由页面在渲染时同步；
/// 页面是数据的唯一来源，表格不单独维护一份可变副本。
pub struct RuleTable {
    items: Rc<Vec<Rule>>,
    rows: Rc<Vec<usize>>,
    rem_size: Pixels,
    page: WeakEntity<RulePage>,
}

impl RuleTable {
    fn new(page: WeakEntity<RulePage>, rem_size: Pixels) -> Self {
        Self {
            items: Rc::new(Vec::new()),
            rows: Rc::new(Vec::new()),
            rem_size,
            page,
        }
    }

    fn set_content(&mut self, items: Rc<Vec<Rule>>, rows: Rc<Vec<usize>>) {
        self.items = items;
        self.rows = rows;
    }

    /// 更新缩放基准，返回是否发生了变化。
    fn set_rem_size(&mut self, rem_size: Pixels) -> bool {
        if self.rem_size == rem_size {
            return false;
        }
        self.rem_size = rem_size;
        true
    }

    /// 第 `row_ix` 行对应的规则，行号是过滤后的位置。
    fn item(&self, row_ix: usize) -> Option<&Rule> {
        self.rows.get(row_ix).map(|ix| &self.items[*ix])
    }

    /// 名称列：名称在上、地址在下，一行就能看出规则指向哪里。
    fn name(&self, item: &Rule, cx: &App) -> AnyElement {
        let url = if item.url.is_empty() {
            "本地规则".to_string()
        } else {
            item.url.clone()
        };

        v_flex()
            .gap_0p5()
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .child(item.name.clone()),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(url),
            )
            .into_any_element()
    }

    /// 操作列：同步、编辑、删除。
    ///
    /// 三个都是图标按钮，用 tooltip 说明含义，并以 `accessibility_label` 给出可读名称。
    fn action(&self, item: &Rule, cx: &App) -> AnyElement {
        let page = self.page.clone();
        let id = item.id.clone();
        let name = item.name.clone();
        // 按钮标识跟随规则 id：过滤后行号会变，用行号会串掉按钮的内部状态。
        let sync_id: ElementId = format!("rule-sync:{}", item.id).into();
        let edit_id: ElementId = format!("rule-edit:{}", item.id).into();
        let delete_id: ElementId = format!("rule-delete:{}", item.id).into();

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

impl TableDelegate for RuleTable {
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
            0 => self.name(item, cx),
            1 => div()
                .text_sm()
                .child(item.count.to_string())
                .into_any_element(),
            2 => div().text_sm().child(kind(item)).into_any_element(),
            3 => div()
                .text_sm()
                .child(format::since(item.refresh_time))
                .into_any_element(),
            _ => self.action(item, cx),
        }
    }
}

/// 规则类型：地址为空即本地规则，否则是远端规则。
fn kind(item: &Rule) -> &'static str {
    if item.url.is_empty() {
        "本地"
    } else {
        "远程"
    }
}
