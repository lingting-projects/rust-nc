mod dialog;
mod table;
mod toolbar;

use std::rc::Rc;

use gpui_kit::base::{StyledExt as _, v_flex};
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::input::InputState;
use gpui_kit::component::notification::Notification;
use gpui_kit::component::table::TableState;
use gpui_kit::*;

use lingting_gpui_kit::card::Card;

use crate::services::{self, AppServices};
use crate::utils::refresh::{Refresher, TICK};
use table::SubscribeTable;
use types_core::dto::subscribe::SubscribeUpsertDTO;
use types_core::entity::subscribe::Subscribe;

/// 页面内各卡片的内边距，搜索栏与表格区域共用，保证留白一致。
pub(crate) const PADDING: Rems = rems(1.);

/// 页面的加载状态：读取中、已就绪或失败原因。
pub enum LoadState {
    Loading,
    Ready(Rc<Vec<Subscribe>>),
    Failed(SharedString),
}

/// 订阅页面：搜索栏加订阅表格。
///
/// 数据来自 `service-core`，新增、编辑与删除在写库后重新加载表格；
/// 同步按钮尚未接入数据层，只保留交互反馈。
pub struct SubscribePage {
    load: LoadState,
    /// 输入框中的关键字，为空表示不过滤。
    keyword: String,
    /// 搜索输入框的状态。
    query: Entity<InputState>,
    /// 表格状态，页面存活期间一直保留。
    table: Entity<TableState<SubscribeTable>>,
    /// 输入变化订阅，页面存活期间一直保留。
    _query_subscription: Subscription,
    /// 是否正在同步全部订阅。
    syncing: bool,
    /// 是否正在提交写操作，避免重复提交。
    submitting: bool,
    /// 刷新中状态与轮询节奏。
    refresh: Refresher,
    /// 轮询任务，只在轮询期间存在。
    _ticker: Option<Task<()>>,
}

impl SubscribePage {
    /// 创建页面并立即发起首次加载。
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query = toolbar::create_state(window, cx);
        let query_subscription = toolbar::subscribe(&query, cx);
        let table = table::create(window, cx);

        let mut page = Self {
            load: LoadState::Loading,
            keyword: String::new(),
            query,
            table,
            _query_subscription: query_subscription,
            syncing: false,
            submitting: false,
            refresh: Refresher::new(),
            _ticker: None,
        };
        page.reload(cx);
        page
    }

    /// 重新读取订阅列表。
    pub fn reload(&mut self, cx: &mut Context<Self>) {
        let services = match services::get(cx) {
            Some(AppServices::Ready(services)) => services.clone(),
            Some(AppServices::Failed(reason)) => {
                self.load = LoadState::Failed(reason.clone());
                return;
            }
            None => {
                self.load = LoadState::Failed("数据层未初始化".into());
                return;
            }
        };

        self.load = LoadState::Loading;
        cx.notify();

        let loader = services.clone();
        let runner = services.clone();
        cx.spawn(async move |this, cx| {
            let data = loader.spawn(async move { runner.subscribe_list().await });
            let result = match data.await {
                Ok(Ok(data)) => Ok(data),
                Ok(Err(error)) => Err(format!("{error:#}")),
                Err(error) => Err(format!("读取任务异常: {error}")),
            };

            this.update(cx, |page, cx| {
                page.load = match result {
                    Ok(data) => LoadState::Ready(Rc::new(data)),
                    Err(reason) => LoadState::Failed(reason.into()),
                };
                // 首次进入或手动查询时列表可能已在后台刷新，状态要跟着调度器走
                page.refresh_pending(cx);
            })
            .ok();
        })
        .detach();
    }

    /// 更新搜索关键字。表格按关键字过滤，数据本身不变。
    pub fn set_keyword(&mut self, keyword: String, cx: &mut Context<Self>) {
        if self.keyword != keyword {
            self.keyword = keyword;
            cx.notify();
        }
    }

    /// 打开新增订阅的弹窗。
    pub fn create(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        dialog::open(self, None, window, cx);
    }

    /// 打开编辑订阅的弹窗。
    pub fn edit(&mut self, id: i64, window: &mut Window, cx: &mut Context<Self>) {
        let draft = self
            .subscribes()
            .and_then(|items| items.iter().find(|item| item.id == id))
            .map(dialog::SubscribeDraft::of);
        dialog::open(self, draft, window, cx);
    }

    /// 提交新增或修改。成功后关闭弹窗并刷新列表。
    ///
    /// 新增与修改走业务层的不同流程，写库成功后由业务层决定是否触发同步。
    pub fn submit(&mut self, dto: SubscribeUpsertDTO, window: &mut Window, cx: &mut Context<Self>) {
        if self.submitting {
            return;
        }
        let Some(services) = self.ready_services(cx) else {
            return;
        };

        self.submitting = true;
        cx.notify();

        // 无 id 视为新增，两者在业务层是不同流程：新增总会触发同步，修改只在地址变更后触发
        let create = dto.id.is_none();
        let loader = services.clone();
        let runner = services.clone();
        cx.spawn_in(window, async move |this, cx| {
            let result = loader
                .spawn(async move {
                    if create {
                        runner.create_subscribe(dto).await
                    } else {
                        runner.edit_subscribe(dto).await
                    }
                })
                .await;

            this.update_in(cx, |page, window, cx| {
                page.submitting = false;
                match result {
                    Ok(Ok(_)) => {
                        window.close_dialog(cx);
                        page.reload(cx);
                    }
                    Ok(Err(error)) => {
                        page.failed(window, cx, format!("保存订阅失败: {error:#}"));
                    }
                    Err(error) => {
                        page.failed(window, cx, format!("保存任务异常: {error}"));
                    }
                }
            })
            .ok();
        })
        .detach();
    }

    /// 删除一条订阅，调用方负责先取得确认。
    pub fn remove(&mut self, id: i64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(services) = self.ready_services(cx) else {
            return;
        };

        let loader = services.clone();
        let runner = services.clone();
        cx.spawn_in(window, async move |this, cx| {
            let result = loader
                .spawn(async move { runner.delete_subscribe(id).await })
                .await;

            this.update_in(cx, |page, window, cx| match result {
                Ok(Ok(())) => page.reload(cx),
                Ok(Err(error)) => page.failed(window, cx, format!("删除订阅失败: {error:#}")),
                Err(error) => page.failed(window, cx, format!("删除任务异常: {error}")),
            })
            .ok();
        })
        .detach();
    }

    /// 同步全部订阅。
    ///
    /// 只把订阅推送给同步调度器，实际拉取与保存由后台任务按并行上限完成，
    /// 因此这里推送成功即可结束按钮的加载状态。
    pub fn sync_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.syncing {
            return;
        }
        let Some(services) = self.ready_services(cx) else {
            return;
        };

        self.syncing = true;
        cx.notify();

        let loader = services.clone();
        let runner = services.clone();
        cx.spawn_in(window, async move |this, cx| {
            let result = loader
                .spawn(async move { runner.sync_subscribe(None).await })
                .await;

            this.update_in(cx, |page, window, cx| {
                page.syncing = false;
                match result {
                    Ok(Ok(count)) => {
                        let message = format!("已推送 {count} 个订阅同步");
                        window.push_notification(Notification::success(message), cx);
                    }
                    Ok(Err(error)) => page.failed(window, cx, format!("同步订阅失败: {error:#}")),
                    Err(error) => page.failed(window, cx, format!("同步任务异常: {error}")),
                }
                // 推送成功后刷新中状态由调度器决定，这里先读一次并开始轮询
                page.refresh_pending(cx);
            })
            .ok();
        })
        .detach();
    }

    /// 同步一条订阅，等待该订阅同步结束后刷新列表。
    pub fn sync_one(&mut self, id: i64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(services) = self.ready_services(cx) else {
            return;
        };

        self.syncing = true;
        cx.notify();

        let loader = services.clone();
        let runner = services.clone();
        cx.spawn_in(window, async move |this, cx| {
            let result = loader
                .spawn(async move { runner.sync_subscribe(Some(id)).await })
                .await;

            this.update_in(cx, |page, window, cx| {
                page.syncing = false;
                match result {
                    Ok(Ok(_)) => page.reload(cx),
                    Ok(Err(error)) => page.failed(window, cx, format!("同步订阅失败: {error:#}")),
                    Err(error) => page.failed(window, cx, format!("同步任务异常: {error}")),
                }
                page.refresh_pending(cx);
            })
            .ok();
        })
        .detach();
    }

    /// 当前表格要展示的订阅。
    pub fn subscribes(&self) -> Option<&[Subscribe]> {
        match &self.load {
            LoadState::Ready(data) => Some(data),
            _ => None,
        }
    }

    /// 搜索输入框的状态。
    pub fn query(&self) -> Entity<InputState> {
        self.query.clone()
    }

    /// 是否正在同步。
    pub fn is_syncing(&self) -> bool {
        self.syncing
    }

    /// 是否正在提交写操作。
    pub fn is_submitting(&self) -> bool {
        self.submitting
    }
    /// 同步刷新中状态，并在需要时开始轮询。
    ///
    /// 同步只推送任务，真正的拉取在后台进行，因此表格的「刷新中」只能问调度器。
    /// 每次调用都会把轮询至少再维持一段时间，避免任务刚入队时状态一闪而过。
    pub fn refresh_pending(&mut self, cx: &mut Context<Self>) {
        let Some(services) = self.ready_services(cx) else {
            return;
        };

        if self.refresh.update(services.subscribe_pending()) {
            cx.notify();
        }
        self.poll(cx);
    }

    /// 按固定间隔重查刷新状态，直到没有数据在刷新且过了最短持续时间。
    ///
    /// 同一时刻只存在一个轮询任务；结束时会清掉句柄与轮询标记，
    /// 下一次同步再重新派生。
    fn poll(&mut self, cx: &mut Context<Self>) {
        if !self.refresh.begin() {
            return;
        }
        // 旧任务已结束，替换成新的；旧任务结束时不会再动这个句柄
        self._ticker = None;
        self._ticker = Some(cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(TICK).await;

                let alive = this
                    .update(cx, |page, cx| {
                        let Some(services) = page.ready_services(cx) else {
                            return false;
                        };
                        if page.refresh.tick(services.subscribe_pending()) {
                            cx.notify();
                        }
                        let alive = page.refresh.alive();
                        if !alive {
                            page.refresh.stop();
                        }
                        alive
                    })
                    .unwrap_or(false);
                if !alive {
                    // 句柄留到下一次轮询时被替换，避免任务在这里丢掉自己
                    return;
                }
            }
        }));
    }

    /// 写操作失败时的一处反馈：通知 + 让依赖它的按钮退出加载态。
    fn failed(&mut self, window: &mut Window, cx: &mut Context<Self>, reason: String) {
        window.push_notification(Notification::error(reason), cx);
        cx.notify();
    }

    /// 取数据层；未就绪时返回 `None`。
    fn ready_services(&self, cx: &App) -> Option<std::sync::Arc<services::Services>> {
        match services::get(cx) {
            Some(AppServices::Ready(services)) => Some(services.clone()),
            _ => None,
        }
    }
}

impl Render for SubscribePage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 每帧把页面数据、过滤结果与当前缩放同步给表格：页面是数据的唯一来源。
        if let LoadState::Ready(items) = &self.load {
            let rows = table::matched(items, &self.keyword);
            table::sync(
                &self.table,
                items.clone(),
                Rc::new(rows),
                self.refresh.pending(),
                window.rem_size(),
                cx,
            );
        }

        let content = match &self.load {
            LoadState::Loading => table::placeholder("正在加载订阅…", cx),
            LoadState::Failed(reason) => table::placeholder(reason, cx),
            LoadState::Ready(_) => table::render(&self.table),
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
    }
}
