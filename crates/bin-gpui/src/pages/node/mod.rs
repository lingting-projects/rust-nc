mod delay;
mod groups;
mod nodes;

use std::collections::HashSet;
use std::time::Duration;

use gpui_kit::base::{StyledExt as _, h_flex, v_flex};
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::input::InputState;
use gpui_kit::component::notification::Notification;
use gpui_kit::*;

use lib_biz::kernel::{SingBoxInstance, Status};

use crate::blocks::KernelOffline;
use crate::services::{self, AppServices, GroupInfo, GroupNode};
use delay::Delay;

/// 页面内各卡片的内边距，左栏标题区与右栏搜索区共用，保证留白一致。
pub(crate) const PADDING: Rems = rems(1.);

/// 自动刷新间隔。测速结果由内核异步推送，界面按固定间隔重新读取分组即可跟上。
const REFRESH_INTERVAL: Duration = Duration::from_secs(1);

/// 页面的加载状态：读取中、已就绪或失败原因。
pub enum LoadState {
    Loading,
    Ready(Vec<GroupInfo>),
    Failed(SharedString),
}

/// 节点页面：左侧分组列表，右侧选中分组的节点列表。
///
/// 全部数据来自 sing-box 内核 gRPC API：分组与节点延迟读自 `SubscribeGroups`，
/// 切换节点走 `SelectOutbound`，测速走 `URLTest`。内核未运行时页面只提示去控制台启动。
pub struct NodePage {
    load: LoadState,
    /// 当前选中的分组标签。
    selected: Option<String>,
    /// 分组搜索框中的关键字。
    group_keyword: String,
    /// 节点搜索框中的关键字。
    node_keyword: String,
    group_query: Entity<InputState>,
    node_query: Entity<InputState>,
    /// 分组搜索框的输入订阅，页面存活期间一直保留。
    _group_subscription: Subscription,
    /// 节点搜索框的输入订阅，页面存活期间一直保留。
    _node_subscription: Subscription,
    /// 是否正在手动刷新分组。
    refreshing: bool,
    /// 正在测速的分组标签，按钮据此保持加载态。
    testing: HashSet<String>,
    /// 每秒重新读取分组、同步内核状态的任务，页面存活期间一直保留。
    _ticker: Task<()>,
}

impl NodePage {
    /// 创建页面并启动自动刷新。
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let group_query = groups::create_state(window, cx);
        let group_subscription = groups::subscribe(&group_query, cx);
        let node_query = nodes::create_state(window, cx);
        let node_subscription = nodes::subscribe(&node_query, cx);

        let mut page = Self {
            load: LoadState::Loading,
            selected: None,
            group_keyword: String::new(),
            node_keyword: String::new(),
            group_query,
            node_query,
            _group_subscription: group_subscription,
            _node_subscription: node_subscription,
            refreshing: false,
            testing: HashSet::new(),
            _ticker: cx.spawn(async move |this, cx| {
                loop {
                    cx.background_executor().timer(REFRESH_INTERVAL).await;
                    if this.update(cx, |page, cx| page.refresh(cx)).is_err() {
                        return;
                    }
                }
            }),
        };

        page.reload_now(cx);
        page
    }

    /// 手动刷新：按钮进入加载态，读取结束后退出。
    pub fn reload(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.refreshing = true;
        cx.notify();
        self.reload_now(cx);
    }

    /// 重新读取出站分组。
    ///
    /// 自动刷新走这里：读取期间不动按钮状态，读取失败也不覆盖已有数据，
    /// 避免内核短暂不可用时列表被清空。
    fn reload_now(&mut self, cx: &mut Context<Self>) {
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

        let loader = services.clone();
        let runner = services.clone();
        cx.spawn(async move |this, cx| {
            let result = loader
                .spawn(async move { runner.kernel_groups().await })
                .await;

            this.update(cx, |page, cx| {
                page.refreshing = false;
                match result {
                    Ok(Ok(groups)) => {
                        page.apply(groups);
                    }
                    Ok(Err(error)) => page.failed(format!("读取节点分组失败: {error:#}")),
                    Err(error) => page.failed(format!("读取节点分组任务异常: {error}")),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// 自动刷新：重读分组，失败时保留原内容。
    fn refresh(&mut self, cx: &mut Context<Self>) {
        // 加载中已有请求在飞，不重复轮询；已就绪与首次失败都跟随内核状态重读，
        // 内核从控制台启动后页面会自行恢复。
        let pending = matches!(self.load, LoadState::Loading);
        if pending || SingBoxInstance::instance().status() != Status::Running {
            return;
        }

        let Some(services) = services::ready(cx) else {
            return;
        };

        let loader = services.clone();
        let runner = services.clone();
        cx.spawn(async move |this, cx| {
            let result = loader
                .spawn(async move { runner.kernel_groups().await })
                .await;

            // 自动刷新不提示失败：下一次轮询大概率就能读到
            if let Ok(Ok(groups)) = result {
                this.update(cx, |page, cx| {
                    if page.apply(groups) {
                        cx.notify();
                    }
                })
                .ok();
            }
        })
        .detach();
    }

    /// 用新数据替换列表，返回数据是否发生变化。
    ///
    /// 选中分组在刷新后保留；选中的分组消失时退回第一个分组，保证右栏总有内容。
    fn apply(&mut self, groups: Vec<GroupInfo>) -> bool {
        let selected = self
            .selected
            .clone()
            .filter(|tag| groups.iter().any(|group| &group.tag == tag))
            .or_else(|| groups.first().map(|group| group.tag.clone()));

        let changed = selected != self.selected || !same_groups(&self.load, &groups);
        self.selected = selected;
        self.load = LoadState::Ready(groups);

        // 测速完成的判据：分组仍存在，且选中节点的延迟已不再是「未测速」。
        // 只按分组存在与否判断的话，加载态会一直停在按钮上。
        let testing = &mut self.testing;
        match &self.load {
            LoadState::Ready(groups) => testing.retain(|tag| {
                groups
                    .iter()
                    .find(|group| &group.tag == tag)
                    .is_some_and(|group| Delay::of(group.delay()) == Delay::Unknown)
            }),
            _ => testing.clear(),
        }

        changed
    }

    /// 读取失败：首次加载失败才替换内容，已有数据时只提示。
    fn failed(&mut self, reason: String) {
        if matches!(self.load, LoadState::Ready(_)) {
            log::warn!("[节点] {reason}");
        } else {
            self.load = LoadState::Failed(reason.into());
        }
    }

    /// 切换选中的分组。
    pub fn select_group(&mut self, tag: &str, cx: &mut Context<Self>) {
        if self.selected.as_deref() == Some(tag) {
            return;
        }
        self.selected = Some(tag.to_string());
        cx.notify();
    }

    /// 在手动选择分组里切换选中节点。
    ///
    /// 自动选择分组由内核按延迟自动挑选，这里不做任何事。
    pub fn switch_node(&mut self, tag: &str, cx: &mut Context<Self>) {
        let (Some(group), Some(services)) =
            (self.group().map(|g| g.tag.clone()), services::ready(cx))
        else {
            return;
        };
        if !self.switchable() {
            return;
        }

        let tag = tag.to_string();
        let loader = services.clone();
        let runner = services.clone();
        cx.spawn(async move |this, cx| {
            let result = loader
                .spawn(async move { runner.kernel_select(&group, &tag).await })
                .await;

            this.update(cx, |page, cx| match result {
                Ok(Ok(())) => {
                    // 切换成功后立刻回读一次，选中标记不必等下一次轮询
                    page.reload_now(cx);
                }
                Ok(Err(error)) => page.notify_error(format!("切换节点失败: {error:#}"), cx),
                Err(error) => page.notify_error(format!("切换节点任务异常: {error}"), cx),
            })
            .ok();
        })
        .detach();
    }

    /// 对当前分组的选中节点测速。
    ///
    /// 队列按顺序逐个测速并自行去重，因此这里只负责推送；结果由自动刷新同步。
    pub fn url_test(&mut self, group_tag: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(services) = services::ready(cx) else {
            return;
        };
        let Some(tag) = self
            .groups()
            .and_then(|groups| groups.iter().find(|group| group.tag == group_tag))
            .map(|group| group.selected.clone())
            .filter(|tag| !tag.is_empty())
        else {
            window.push_notification(Notification::warning("该分组没有可测速的节点"), cx);
            return;
        };

        match services.kernel_url_test(&tag) {
            // 入队成功与「已在队列中」都是排队状态，按钮一律进入加载态；
            // 延迟由自动刷新带回，加载态在测到结果后自动退出。
            Ok(_) => {
                self.testing.insert(group_tag.to_string());
                cx.notify();
            }
            Err(error) => self.notify_error(format!("测速失败: {error:#}"), cx),
        }
    }

    /// 分组列表，未就绪时为 `None`。
    pub fn groups(&self) -> Option<&[GroupInfo]> {
        match &self.load {
            LoadState::Ready(groups) => Some(groups),
            _ => None,
        }
    }

    /// 当前选中的分组。
    pub fn group(&self) -> Option<&GroupInfo> {
        let tag = self.selected.as_ref()?;
        self.groups()?.iter().find(|group| &group.tag == tag)
    }

    /// 当前分组的节点列表。
    pub fn nodes(&self) -> Option<&[GroupNode]> {
        self.group().map(|group| group.items.as_slice())
    }

    /// 当前分组里选中的节点标签。
    pub fn current(&self) -> Option<&str> {
        self.group()
            .map(|group| group.selected.as_str())
            .filter(|selected| !selected.is_empty())
    }

    /// 当前分组是否支持手动切换节点。
    pub fn switchable(&self) -> bool {
        self.group().is_some_and(|group| group.selectable)
    }

    /// 当前选中的分组标签。
    pub fn selected(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    /// 首次加载失败的原因。
    pub fn failure(&self) -> Option<&str> {
        match &self.load {
            LoadState::Failed(reason) => Some(reason),
            _ => None,
        }
    }

    /// 分组搜索关键字。
    pub fn group_keyword(&self) -> &str {
        &self.group_keyword
    }

    /// 节点搜索关键字。
    pub fn node_keyword(&self) -> &str {
        &self.node_keyword
    }

    /// 分组搜索框状态。
    pub fn group_query(&self) -> Entity<InputState> {
        self.group_query.clone()
    }

    /// 节点搜索框状态。
    pub fn node_query(&self) -> Entity<InputState> {
        self.node_query.clone()
    }

    /// 是否正在手动刷新。
    pub fn is_refreshing(&self) -> bool {
        self.refreshing
    }

    /// 正在测速的分组快照，供列表行判断按钮的加载态。
    pub fn testing_snapshot(&self) -> HashSet<String> {
        self.testing.clone()
    }

    /// 更新分组搜索关键字。
    pub fn set_group_keyword(&mut self, keyword: String, cx: &mut Context<Self>) {
        if self.group_keyword != keyword {
            self.group_keyword = keyword;
            cx.notify();
        }
    }

    /// 更新节点搜索关键字。
    pub fn set_node_keyword(&mut self, keyword: String, cx: &mut Context<Self>) {
        if self.node_keyword != keyword {
            self.node_keyword = keyword;
            cx.notify();
        }
    }

    /// 操作失败时的一处反馈：通知 + 让依赖它的按钮退出加载态。
    fn notify_error(&mut self, reason: String, cx: &mut Context<Self>) {
        log::warn!("[节点] {reason}");
        self.testing.clear();
        cx.notify();
    }
}

/// 两轮数据是否完全一致，用于判断自动刷新是否需要重绘。
fn same_groups(current: &LoadState, groups: &[GroupInfo]) -> bool {
    let LoadState::Ready(current) = current else {
        return false;
    };
    current.len() == groups.len()
        && current.iter().zip(groups).all(|(left, right)| {
            left.tag == right.tag
                && left.selected == right.selected
                && left.selectable == right.selectable
                && left.items.len() == right.items.len()
                && left
                    .items
                    .iter()
                    .zip(&right.items)
                    .all(|(a, b)| a.tag == b.tag && a.delay == b.delay && a.kind == b.kind)
        })
}

impl Render for NodePage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 内核未运行时没有可读的分组，页面整体换成启动提示。
        if SingBoxInstance::instance().status() != Status::Running {
            return v_flex()
                .size_full()
                .min_h_0()
                .child(KernelOffline::render(cx))
                .into_any_element();
        }

        h_flex()
            .size_full()
            .min_h_0()
            .gap_4()
            .child(groups::render(self, cx))
            .child(nodes::render(self, cx))
            .into_any_element()
    }
}
