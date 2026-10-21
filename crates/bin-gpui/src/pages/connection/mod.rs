mod dialog;
mod filter;
mod rows;
mod table;
mod toolbar;

use std::rc::Rc;
use std::time::Duration;

use gpui_kit::base::{StyledExt as _, v_flex};
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::input::InputState;
use gpui_kit::component::notification::Notification;
use gpui_kit::component::table::TableState;
use gpui_kit::*;

use lib_biz::kernel::{SingBoxInstance, Status};
use lingting_gpui_kit::card::Card;

use crate::blocks::KernelOffline;
use crate::services::{self, AppServices, ConnectionInfo};

use filter::Status as FilterStatus;
use rows::{ConnectionList, ConnectionRow};
use table::ConnectionTable;

/// 页面内各卡片的内边距，搜索栏与表格区域共用，保证留白一致。
pub(crate) const PADDING: Rems = rems(1.);

/// 轮询间隔。连接与累计流量都由内核异步推送，界面按固定间隔重新读取即可跟上。
const REFRESH_INTERVAL: Duration = Duration::from_secs(1);

/// 页面的加载状态：读取中、已就绪或失败原因。
pub enum LoadState {
    Loading,
    Ready,
    Failed(SharedString),
}

/// 连接页面：上方搜索栏，下方连接表格。
///
/// 数据来自内核 gRPC API 的 `SubscribeConnections`：每秒读取一次连接快照，
/// 用累计流量的两次采样差算出速率；断开连接走 `CloseConnection`。
/// 内核只推送仍处于打开状态的连接，已关闭的连接由页面自行记录。
/// 内核未运行时页面只提示去控制台启动。
pub struct ConnectionPage {
    load: LoadState,
    /// 当前的状态筛选，默认只展示活动连接。
    status: FilterStatus,
    /// 搜索框中的关键字。
    keyword: String,
    /// 是否暂停轮询：暂停后保留已读到的数据，不再更新。
    paused: bool,
    /// 活动与已关闭的连接，按轮询结果持续更新。
    list: ConnectionList,
    /// 表格当前要展示的行下标。只有数据、筛选或关键字变化时才重算，
    /// 否则每帧都会得到一个新的 `Rc`，表格会误判为内容变化而持续重绘。
    visible: Rc<Vec<usize>>,
    query: Entity<InputState>,
    table: Entity<TableState<ConnectionTable>>,
    /// 搜索框的输入订阅，页面存活期间一直保留。
    _query_subscription: Subscription,
    /// 每秒重新读取连接的任务，页面存活期间一直保留。
    _ticker: Task<()>,
}

impl ConnectionPage {
    /// 创建页面并启动轮询。
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query = toolbar::create_state(window, cx);
        let query_subscription = toolbar::subscribe(&query, cx);
        let table = table::create(window, cx);

        let mut page = Self {
            load: LoadState::Loading,
            status: FilterStatus::Active,
            keyword: String::new(),
            paused: false,
            list: ConnectionList::new(),
            visible: Rc::new(Vec::new()),
            query,
            table,
            _query_subscription: query_subscription,
            _ticker: cx.spawn(async move |this, cx| {
                loop {
                    cx.background_executor().timer(REFRESH_INTERVAL).await;
                    if this.update(cx, |page, cx| page.poll(cx)).is_err() {
                        return;
                    }
                }
            }),
        };

        page.reload(cx);
        page
    }

    /// 立即重新读取一次连接，用于首次加载与手动恢复。
    fn reload(&mut self, cx: &mut Context<Self>) {
        self.load = LoadState::Loading;
        cx.notify();
        self.poll(cx);
    }

    /// 读取一次连接快照，内核未运行或暂停时直接返回。
    ///
    /// 读取失败不覆盖已有数据：内核短暂不可用时列表应当保持可见，
    /// 只有首次加载失败才把页面换成失败原因。
    fn poll(&mut self, cx: &mut Context<Self>) {
        if self.paused || SingBoxInstance::instance().status() != Status::Running {
            return;
        }

        let Some(services) = services::ready(cx) else {
            if matches!(self.load, LoadState::Loading) {
                self.load = LoadState::Failed("数据层未初始化".into());
                cx.notify();
            }
            return;
        };

        let loader = services.clone();
        let runner = services.clone();
        cx.spawn(async move |this, cx| {
            let result = loader
                .spawn(async move { runner.kernel_connections().await })
                .await;

            this.update(cx, |page, cx| match result {
                Ok(Ok(connections)) => page.apply(connections, cx),
                Ok(Err(error)) => page.failed(format!("读取连接失败: {error:#}"), cx),
                Err(error) => page.failed(format!("读取连接任务异常: {error}"), cx),
            })
            .ok();
        })
        .detach();
    }

    /// 用新的连接快照替换列表。
    fn apply(&mut self, connections: Vec<ConnectionInfo>, cx: &mut Context<Self>) {
        self.list.update(connections);
        self.refresh_visible();
        self.load = LoadState::Ready;
        cx.notify();
    }

    /// 读取失败：首次加载失败才替换内容，已有数据时只记日志。
    fn failed(&mut self, reason: String, cx: &mut Context<Self>) {
        if matches!(self.load, LoadState::Ready) {
            log::warn!("[连接] {reason}");
            return;
        }
        self.load = LoadState::Failed(reason.into());
        cx.notify();
    }

    /// 切换状态筛选。
    pub fn set_status(&mut self, status: FilterStatus, cx: &mut Context<Self>) {
        if self.status != status {
            self.status = status;
            self.refresh_visible();
            cx.notify();
        }
    }

    /// 更新搜索关键字。表格按关键字过滤，数据本身不变。
    pub fn set_keyword(&mut self, keyword: String, cx: &mut Context<Self>) {
        if self.keyword != keyword {
            self.keyword = keyword;
            self.refresh_visible();
            cx.notify();
        }
    }

    /// 暂停或继续轮询。
    ///
    /// 暂停期间保留当前数据；继续时清掉速率采样，避免用暂停前的累计流量
    /// 除以包含暂停时长的时间间隔，算出明显偏低的速度。
    pub fn toggle_pause(&mut self, cx: &mut Context<Self>) {
        self.paused = !self.paused;
        if !self.paused {
            self.list.reset_speed();
        }
        cx.notify();
    }

    /// 清空当前缓存的数据。
    ///
    /// 只清界面上的数据，不关闭内核里的连接；下一次轮询会重新读回来，
    /// 因此清空后暂停才有「清空并停留」的效果。
    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.list.clear();
        self.refresh_visible();
        cx.notify();
    }

    /// 断开一条连接。
    pub fn close_connection(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(services) = services::ready(cx) else {
            return;
        };

        let id = id.to_string();
        let loader = services.clone();
        let runner = services.clone();
        cx.spawn(async move |this, cx| {
            let result = loader
                .spawn(async move { runner.kernel_close_connection(&id).await })
                .await;

            this.update(cx, |page, cx| match result {
                // 断开成功后立刻回读一次，列表不必等下一次轮询；
                // 回读会重算速率，因此重新读取前清掉旧采样。
                Ok(Ok(())) => {
                    page.list.reset_speed();
                    page.poll(cx);
                }
                Ok(Err(error)) => page.notify_error(format!("断开连接失败: {error:#}"), cx),
                Err(error) => page.notify_error(format!("断开连接任务异常: {error}"), cx),
            })
            .ok();
        })
        .detach();
    }

    /// 打开一条连接的详情弹窗。
    pub fn open_detail(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(row) = self.list.find(id).cloned() else {
            return;
        };
        dialog::open(row, window, cx);
    }

    /// 当前状态筛选。
    pub fn status(&self) -> FilterStatus {
        self.status
    }

    /// 表格没有行时的说明：区分筛选结果为空与确实没有连接。
    pub fn empty_hint(&self) -> &'static str {
        if self.is_empty() {
            return "当前没有连接";
        }
        if !self.keyword.trim().is_empty() {
            return "没有匹配的连接";
        }
        match self.status {
            FilterStatus::Closed => "本次打开页面后还没有连接被关闭",
            _ => "当前没有活动连接",
        }
    }

    /// 搜索框状态。
    pub fn query(&self) -> Entity<InputState> {
        self.query.clone()
    }

    /// 是否已暂停轮询。
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// 当前是否有可清空的数据。
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// 重算表格要展示的行：先按状态筛选，再按关键字过滤。
    ///
    /// 合并列表里活动连接在前、已关闭的连接在后，状态筛选因此只是取其中一段：
    /// 关闭的连接不必再判断，它们的下标一定不小于活动连接的数量。
    fn refresh_visible(&mut self) {
        let items = self.list.rows();
        let active = self.list.active_count();

        self.visible = Rc::new(
            rows::matched(&items, &self.keyword)
                .into_iter()
                .filter(|ix| match self.status {
                    FilterStatus::All => true,
                    FilterStatus::Active => *ix < active,
                    FilterStatus::Closed => *ix >= active,
                })
                .collect(),
        );
    }

    /// 操作失败时的一处反馈。
    fn notify_error(&mut self, reason: String, cx: &mut Context<Self>) {
        log::warn!("[连接] {reason}");
        cx.notify();
    }
}

impl Render for ConnectionPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 内核未运行时没有可读的连接，页面整体换成启动提示。
        if SingBoxInstance::instance().status() != Status::Running {
            return v_flex()
                .size_full()
                .min_h_0()
                .child(KernelOffline::render(cx))
                .into_any_element();
        }

        // 每帧把页面数据、过滤结果与当前缩放同步给表格：页面是数据的唯一来源。
        table::sync(
            &self.table,
            self.list.rows(),
            self.visible.clone(),
            self.empty_hint(),
            window.rem_size(),
            cx,
        );

        let content = match &self.load {
            LoadState::Loading => table::placeholder("正在加载连接…", cx),
            LoadState::Failed(reason) => table::placeholder(reason, cx),
            LoadState::Ready => table::render(&self.table),
        };

        v_flex()
            .size_full()
            .min_h_0()
            .gap_4()
            .child(toolbar::render(self, cx))
            .child(
                Card::new()
                    .size_full()
                    .content_padding(PADDING)
                    .child(content),
            )
            .into_any_element()
    }
}
