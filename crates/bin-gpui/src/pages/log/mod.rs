//! 日志页面。
//!
//! 数据来自内核日志文件：`lib-singbox` 的常驻线程增量读取 `kernel.log`，
//! 把「首行 + 续行」解析成块并累积成一份不可变快照，这里每次刷新只取共享引用。
//! 内核未运行时页面只提示去控制台启动，因为内核日志随内核启动一起重写。

mod card;
mod dialog;
mod filter;

use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use gpui_kit::base::{StyledExt as _, v_flex};
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::select::{SearchableVec, SelectState};
use gpui_kit::*;

use lib_biz::kernel::{SingBoxInstance, Status};
use lib_singbox::{LogLevel, LogSnapshot};
use lingting_gpui_kit::card::Card;
use lingting_gpui_kit::virtual_card::VirtualCard;

use crate::blocks::KernelOffline;
use crate::services;
use crate::utils::window::window_state;
use filter::LevelOption;

/// 页面内各卡片的内边距。
pub(crate) const PADDING: Rems = rems(1.);

/// 刷新间隔。读取线程按更短的间隔读文件，界面每秒取一次快照即可。
/// 窗口隐藏时轮询挂起，恢复可见后立即刷新一次。
const REFRESH_INTERVAL: Duration = Duration::from_secs(1);

/// 日志页面：上方筛选栏，下方日志块列表。
pub struct LogPage {
    /// 搜索框中的关键字。
    keyword: String,
    /// 当前的等级筛选，`None` 表示不限等级。
    level: Option<LogLevel>,
    /// 是否暂停刷新：暂停后保留已读到的数据，不再更新。
    paused: bool,
    /// 是否保持在底部：开启时新数据到达就滚到最新一块。
    follow: bool,
    /// 列表的滚动句柄，注入给虚拟列表，用于保持在底部。
    scroll: UniformListScrollHandle,
    /// 当前展示的快照，直接持有共享引用。
    snapshot: Arc<LogSnapshot>,
    /// 列表当前要展示的块。只有数据、等级或关键字变化时才重算，
    /// 否则每帧都会得到一个新的 `Rc`，列表会误判为内容变化而持续重绘。
    visible: Rc<Vec<usize>>,
    query: Entity<InputState>,
    select: Entity<SelectState<SearchableVec<LevelOption>>>,
    /// 搜索框的输入订阅，页面存活期间一直保留。
    _query_subscription: Subscription,
    /// 等级下拉框的选择订阅。
    _select_subscription: Subscription,
    /// 按间隔取快照的任务，页面存活期间一直保留；窗口隐藏时挂起。
    _ticker: Task<()>,
}

impl LogPage {
    /// 创建页面并启动刷新。
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query = cx.new(|cx| InputState::new(window, cx).placeholder("搜索日志"));
        let query_subscription = cx.subscribe(&query, |page, state, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                let keyword = state.read(cx).value().to_string();
                page.set_keyword(keyword, cx);
            }
        });
        let select = filter::create_state(window, cx);
        let select_subscription = filter::subscribe(&select, cx);

        let mut page = Self {
            keyword: String::new(),
            level: None,
            paused: false,
            // 默认跟随：日志页最常见的用法就是盯着最新输出
            follow: true,
            scroll: UniformListScrollHandle::new(),
            snapshot: Arc::default(),
            visible: Rc::new(Vec::new()),
            query,
            select,
            _query_subscription: query_subscription,
            _select_subscription: select_subscription,
            _ticker: cx.spawn(async move |this, cx| {
                loop {
                    window_state().tick(cx, REFRESH_INTERVAL).await;
                    if this.update(cx, |page, cx| page.poll(cx)).is_err() {
                        return;
                    }
                }
            }),
        };

        page.poll(cx);
        page
    }

    /// 取一次当前快照，暂停或内核未运行时直接返回。
    ///
    /// 取快照只是读一个共享引用，不涉及 IO，因此不必放到异步任务里。
    fn poll(&mut self, cx: &mut Context<Self>) {
        if self.paused || SingBoxInstance::instance().status() != Status::Running {
            return;
        }

        let Some(services) = services::ready(cx) else {
            return;
        };

        match services.kernel_logs() {
            Ok(snapshot) => self.apply(snapshot, cx),
            Err(error) => log::warn!("[日志] 读取日志失败: {error:#}"),
        }
    }

    /// 用新的快照整体替换当前数据。
    ///
    /// 只有引用变化才重算展示项：同一份快照被反复取到时列表不需要重绘。
    fn apply(&mut self, snapshot: Arc<LogSnapshot>, cx: &mut Context<Self>) {
        if Arc::ptr_eq(&self.snapshot, &snapshot) {
            return;
        }
        self.snapshot = snapshot;
        self.refresh_visible();
        self.scroll_to_bottom();
        cx.notify();
    }

    /// 保持在底部时把列表滚到最新一块。
    ///
    /// 虚拟列表在下一帧布局时消费这个请求，因此这里只登记目标下标，不涉及实际滚动。
    fn scroll_to_bottom(&self) {
        if !self.follow {
            return;
        }
        let Some(last) = self.visible.len().checked_sub(1) else {
            return;
        };
        self.scroll
            .scroll_to_item_strict(last, ScrollStrategy::Bottom);
    }

    /// 切换是否保持底部。
    ///
    /// 打开时立即滚到最新一块，关闭后新日志不再改变滚动位置。
    pub fn toggle_follow(&mut self, cx: &mut Context<Self>) {
        self.follow = !self.follow;
        self.scroll_to_bottom();
        cx.notify();
    }

    /// 是否保持底部。
    pub fn is_follow(&self) -> bool {
        self.follow
    }

    /// 切换等级筛选。
    pub fn set_level(&mut self, level: Option<LogLevel>, cx: &mut Context<Self>) {
        if self.level != level {
            self.level = level;
            self.refresh_visible();
            // 筛选后列表变短，保持在底部时跟着新的末尾走
            self.scroll_to_bottom();
            cx.notify();
        }
    }

    /// 更新搜索关键字。列表按关键字过滤，数据本身不变。
    pub fn set_keyword(&mut self, keyword: String, cx: &mut Context<Self>) {
        if self.keyword != keyword {
            self.keyword = keyword;
            self.refresh_visible();
            self.scroll_to_bottom();
            cx.notify();
        }
    }

    /// 暂停或继续刷新。
    ///
    /// 暂停期间读取线程照常推进游标，恢复时下一次刷新会用最新快照整体替换。
    pub fn toggle_pause(&mut self, cx: &mut Context<Self>) {
        self.paused = !self.paused;
        if !self.paused {
            self.poll(cx);
        }
        cx.notify();
    }

    /// 打开一个日志块的详情。
    ///
    /// 完整内容按字节偏移在后台读全，不设长度上限；弹窗先展示读取进度，
    /// 关闭后立即中断读取。
    pub fn open_detail(&mut self, start: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(block) = self.snapshot.find(start).cloned() else {
            return;
        };

        dialog::Detail::open(block, window, cx);
    }

    /// 搜索框状态。
    pub fn query(&self) -> Entity<InputState> {
        self.query.clone()
    }

    /// 等级下拉框状态。
    pub fn level(&self) -> Entity<SelectState<SearchableVec<LevelOption>>> {
        self.select.clone()
    }

    /// 是否已暂停刷新。
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// 列表为空时的说明。
    fn empty_hint(&self) -> &'static str {
        if self.snapshot.is_empty() {
            return "内核还没有输出日志";
        }
        if !self.keyword.trim().is_empty() {
            return "没有匹配的日志";
        }
        match self.level {
            Some(_) => "当前等级下没有日志",
            None => "没有可展示的日志",
        }
    }

    /// 重算列表要展示的块下标。
    fn refresh_visible(&mut self) {
        self.visible = Rc::new(filter::filter(&self.snapshot, &self.keyword, self.level));
    }
}

impl Render for LogPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 内核未运行时日志文件已被重写，页面整体换成启动提示。
        if SingBoxInstance::instance().status() != Status::Running {
            return v_flex()
                .size_full()
                .min_h_0()
                .child(KernelOffline::render(cx))
                .into_any_element();
        }

        let list = self.list(cx);

        v_flex()
            .size_full()
            .min_h_0()
            .gap_4()
            .child(filter::render(self, cx))
            .child(div().flex_1().min_h_0().child(list))
            .into_any_element()
    }
}

impl LogPage {
    /// 日志块列表。
    fn list(&self, cx: &mut Context<Self>) -> AnyElement {
        // 只把命中的块交给列表，列表自身不设过滤条件，避免每帧全量匹配。
        let blocks = Rc::new(
            self.visible
                .iter()
                .filter_map(|index| self.snapshot.blocks.get(*index).cloned())
                .collect::<Vec<_>>(),
        );
        let entity = cx.entity();

        VirtualCard::new("log-blocks")
            .with_ghost()
            .items(blocks)
            .item_height(card::ITEM_HEIGHT)
            .scroll_handle(self.scroll.clone())
            .item_renderer(Rc::new(move |block, _, cx| {
                card::render(&entity, block, cx)
            }))
            .empty(
                Card::new().size_full().content_padding(PADDING).child(
                    div()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(self.empty_hint()),
                ),
            )
            .into_any_element()
    }
}
