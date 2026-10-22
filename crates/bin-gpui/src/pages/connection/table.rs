use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::assets::IconName;
use gpui_kit::base::{StyledExt as _, h_flex, v_flex};
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants as _;
use gpui_kit::component::table::{Column, DataTable, TableDelegate, TableState};
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use lib_singbox::{ConnectionRecord, ConnectionSnapshot};

use super::ConnectionPage;
use super::rows;
use crate::utils::format;

/// 各列宽度，单位为 rem，随界面缩放一起变化。
///
/// 操作列固定在左侧，其余列按表格剩余宽度排布并支持横向滚动。
/// 操作列只放两个小号图标按钮（各 1.5rem）加 0.25rem 间隔，
/// 列宽 4.5rem 扣掉单元格左右内边距后还剩约 0.5rem 留白。
const ACTION_WIDTH: Rems = rems(4.5);
const INBOUND_WIDTH: Rems = rems(8.);
const PROCESS_WIDTH: Rems = rems(10.);
const DESTINATION_WIDTH: Rems = rems(16.);
const RULE_WIDTH: Rems = rems(10.);
const CHAIN_WIDTH: Rems = rems(14.);
const SPEED_WIDTH: Rems = rems(10.);
const SOURCE_WIDTH: Rems = rems(12.);

/// 列定义：键、表头文案、宽度。
const COLUMNS: [(&str, &str, Rems); 8] = [
    ("action", "操作", ACTION_WIDTH),
    ("inbound", "入口", INBOUND_WIDTH),
    ("process", "进程", PROCESS_WIDTH),
    ("destination", "目标", DESTINATION_WIDTH),
    ("rule", "规则", RULE_WIDTH),
    ("chain", "链路", CHAIN_WIDTH),
    ("speed", "速度", SPEED_WIDTH),
    ("source", "源地址", SOURCE_WIDTH),
];

/// 建立表格状态。
pub fn create(
    window: &mut Window,
    cx: &mut Context<ConnectionPage>,
) -> Entity<TableState<ConnectionTable>> {
    let page = cx.entity().downgrade();
    let rem_size = window.rem_size();

    cx.new(|cx| {
        TableState::new(ConnectionTable::new(page, rem_size), window, cx)
            .col_movable(false)
            .col_resizable(false)
            .sortable(false)
            .row_selectable(false)
            .col_selectable(false)
    })
}

/// 把页面的快照、过滤结果与当前缩放同步给表格。
///
/// 每帧调用，内容因此总是与页面一致；只有缩放变化时才重建列宽，
/// 因为列宽由 rem 换算而来，缩放后必须重算，否则表头与单元格会错位。
pub fn sync(
    table: &Entity<TableState<ConnectionTable>>,
    items: Arc<ConnectionSnapshot>,
    rows: Rc<Vec<usize>>,
    empty_hint: &'static str,
    rem_size: Pixels,
    cx: &mut App,
) {
    table.update(cx, |state, cx| {
        let changed = state.delegate_mut().set_content(items, rows, empty_hint);
        let resized = state.delegate_mut().set_rem_size(rem_size);
        // 委托只持有数据，表格实体本身不会被通知：不主动通知的话，
        // 保留渲染会复用上一帧，列表就停在旧内容上。
        if changed || resized {
            state.refresh(cx);
            cx.notify();
        }
    });
}

/// 渲染表格。
///
/// 表格铺满父容器剩余高度，行由 `DataTable` 内部的虚拟列表渲染并竖向滚动。
pub fn render(table: &Entity<TableState<ConnectionTable>>) -> AnyElement {
    div()
        .size_full()
        .min_h_0()
        .child(DataTable::new(table).bordered(false).small())
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

/// 连接表格的数据与渲染。
///
/// 八列：操作、入口、进程、目标、规则、链路、速度、源地址。
/// 委托只持有数据与过滤结果的共享引用，由页面在渲染时同步；
/// 页面是数据的唯一来源，表格不单独维护一份可变副本。
pub struct ConnectionTable {
    items: Arc<ConnectionSnapshot>,
    rows: Rc<Vec<usize>>,
    /// 没有行时的说明，由页面按当前筛选条件给出。
    empty_hint: &'static str,
    rem_size: Pixels,
    page: WeakEntity<ConnectionPage>,
}

impl ConnectionTable {
    fn new(page: WeakEntity<ConnectionPage>, rem_size: Pixels) -> Self {
        Self {
            items: Arc::default(),
            rows: Rc::new(Vec::new()),
            empty_hint: "",
            rem_size,
            page,
        }
    }

    /// 更新数据、过滤结果与空状态说明，返回是否发生了变化。
    fn set_content(
        &mut self,
        items: Arc<ConnectionSnapshot>,
        rows: Rc<Vec<usize>>,
        empty_hint: &'static str,
    ) -> bool {
        let changed = !Arc::ptr_eq(&self.items, &items)
            || !Rc::ptr_eq(&self.rows, &rows)
            || self.empty_hint != empty_hint;
        self.items = items;
        self.rows = rows;
        self.empty_hint = empty_hint;
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

    /// 第 `row_ix` 行对应的连接，行号是过滤后的位置。
    fn item(&self, row_ix: usize) -> Option<&ConnectionRecord> {
        self.rows.get(row_ix).map(|ix| &self.items.rows[*ix])
    }

    /// 操作列：详情与断开。
    ///
    /// 两个都是图标按钮，用 tooltip 说明含义，并以 `accessibility_label` 给出可读名称。
    /// 只有活动连接才展示断开按钮：已关闭的连接再断开没有意义，
    /// 留一个禁用按钮既占位又让人以为还能点。
    fn action(&self, item: &ConnectionRecord, cx: &App) -> AnyElement {
        let page = self.page.clone();
        let id = item.info.id.clone();
        let closed = item.closed;
        // 按钮标识跟随连接 id：过滤后行号会变，用行号会串掉按钮的内部状态。
        let detail_id: ElementId = format!("connection-detail:{}", item.info.id).into();
        let close_id: ElementId = format!("connection-close:{}", item.info.id).into();

        h_flex()
            .gap_1()
            .child(
                Button::new(detail_id)
                    .ghost()
                    .small()
                    .icon(IconName::Info)
                    .tooltip("详情")
                    .accessibility_label("详情")
                    .on_click({
                        let page = page.clone();
                        let id = id.clone();
                        move |_, window, cx| {
                            page.update(cx, |page, cx| page.open_detail(&id, window, cx))
                                .ok();
                        }
                    }),
            )
            .when(!closed, |this| {
                this.child(
                    Button::new(close_id)
                        .ghost()
                        .small()
                        .icon(IconName::Unplug)
                        .text_color(cx.theme().danger)
                        .tooltip("断开")
                        .accessibility_label("断开")
                        .on_click({
                            let page = page.clone();
                            move |_, _, cx| {
                                page.update(cx, |page, cx| page.close_connection(&id, cx))
                                    .ok();
                            }
                        }),
                )
            })
            .into_any_element()
    }
}

impl TableDelegate for ConnectionTable {
    fn columns_count(&self, _: &App) -> usize {
        COLUMNS.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        self.rows.len()
    }

    fn column(&self, col_ix: usize, _: &App) -> Column {
        let (key, name, width) = COLUMNS[col_ix];
        let column = Column::new(key, name)
            .width(width.to_pixels(self.rem_size))
            .resizable(false)
            .movable(false)
            .selectable(false);

        // 操作列固定在左侧：横向滚动时始终可见。
        match key {
            "action" => column.fixed_left(),
            _ => column,
        }
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
            0 => self.action(item, cx),
            1 => cell(item, cx, item.info.inbound.clone()),
            2 => cell(item, cx, rows::process(item)),
            3 => cell(item, cx, item.info.destination.clone()),
            4 => cell(item, cx, rule(item)),
            5 => cell(item, cx, rows::chain(item)),
            6 => speed(item, cx),
            _ => cell(item, cx, item.info.source.clone()),
        }
    }

    fn render_empty(
        &mut self,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .child(self.empty_hint)
    }
}

/// 普通文本列：已关闭的连接用次要文字色，与活动连接区分开。
fn cell(item: &ConnectionRecord, cx: &App, text: String) -> AnyElement {
    div()
        .truncate()
        .when(item.closed, |this| {
            this.text_color(cx.theme().muted_foreground)
        })
        .child(text)
        .into_any_element()
}

/// 速度列：下载在上、上传在下，各自按自适应单位展示。
///
/// 两行固定排布，即使某一项为 0 也占着自己那一行：只有一个数值时，
/// 看不出它到底是下载还是上传。行高与字距收在同一处，
/// 两行叠起来不高于一行文本在行高里的占位，避免撑破表格行高。
fn speed(item: &ConnectionRecord, cx: &App) -> AnyElement {
    v_flex()
        .line_height(relative(1.))
        .child(
            h_flex()
                .gap_1()
                .child(Icon::new(IconName::ArrowDown).xsmall())
                .child(format::speed(item.download)),
        )
        .child(
            h_flex()
                .gap_1()
                .text_color(cx.theme().muted_foreground)
                .child(Icon::new(IconName::ArrowUp).xsmall())
                .child(format::speed(item.upload)),
        )
        .into_any_element()
}

/// 规则列：未命中规则时内核给出空值，展示为 `-`。
fn rule(item: &ConnectionRecord) -> String {
    if item.info.rule.is_empty() {
        "-".to_string()
    } else {
        item.info.rule.clone()
    }
}
