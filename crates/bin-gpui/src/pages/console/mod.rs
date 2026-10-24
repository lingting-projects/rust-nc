mod charts;
mod failure;
mod monitor;
mod preview;
mod toolbar;

use gpui_kit::base::{StyledExt as _, h_flex, v_flex};
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::select::{SearchableVec, SelectState};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use lib_biz::kernel::Status;
use lingting_gpui_kit::card::{ItemCard, ItemCardSelectedStyle};

use crate::services::{self, AppServices, ConsoleData, KernelData};
use monitor::Monitor;
use toolbar::ConfigOption;

/// 各区域卡片的内边距。
///
/// 控制栏、预览栏、监控控制栏、监控区的数值卡与图表卡共用，保证同一层的留白一致。
pub(crate) const PADDING: Rems = rems(1.);

/// 图表区最小高度。
///
/// 页面整体可滚动后，图表区拿到的剩余高度可能为零，折线会被压成一条线；
/// 给一个下限，窗口变矮时改为整页滚动，图表本身仍然可读。
const CHART_MIN_HEIGHT: Rems = rems(12.);

/// 实时监控卡片的分区，可独立开关。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MonitorSection {
    /// 基础：上下行速率与累计流量。
    Basic,
    /// 流量：上下行速率折线图。
    Traffic,
    /// 内存：内存占用折线图。
    Memory,
}

impl MonitorSection {
    /// 监控控制栏里的分区顺序与名称。
    pub const ALL: [(&'static str, MonitorSection); 3] = [
        ("基础", MonitorSection::Basic),
        ("流量", MonitorSection::Traffic),
        ("内存", MonitorSection::Memory),
    ];
}

/// 内核状态，取自全局内核实例。
pub struct Kernel {
    /// 内核状态。
    pub status: Status,
    /// 本页发起的启停操作是否还没结束。
    pub loading: bool,
    /// 全局是否有启停操作在进行，可能来自托盘或启动时的自动运行。
    pub acting: bool,
    /// 正在运行的配置 id，未运行时为 `None`。
    pub config_id: Option<i64>,
    /// 最近一次异常的原因。
    pub reason: Option<String>,
}

impl Kernel {
    /// 由服务层返回的状态快照构造。
    fn new(data: KernelData) -> Self {
        Self {
            status: data.status,
            loading: false,
            acting: data.acting,
            config_id: data.config_id,
            reason: data.reason,
        }
    }

    /// 是否正在运行。
    pub fn running(&self) -> bool {
        self.status == Status::Running
    }
}

/// 配置与订阅的加载状态。
pub enum LoadState {
    Loading,
    Ready(ConsoleData),
    Failed(SharedString),
}

/// 控制台页面：自上而下四段——控制栏、预览栏、监控控制栏、实时监控。
///
/// 配置与订阅来自 `service-core`，内核状态取自全局内核实例，实时监控取自内核 gRPC API。
pub struct Console {
    load: LoadState,
    kernel: Kernel,
    /// 配置下拉框的状态。
    select: Entity<SelectState<SearchableVec<ConfigOption>>>,
    /// 下拉框中选中的配置 id，未选中时为 `None`。
    selected: Option<i64>,
    /// 是否正在刷新配置列表。
    refreshing: bool,
    /// 监控控制栏中被选中的分区，默认全部选中。
    sections: Vec<MonitorSection>,
    monitor: Monitor,
    /// 每秒推进一次监控数据的任务，页面存活期间一直保留。
    _ticker: Task<()>,
    /// 下拉框的选择事件订阅，页面存活期间一直保留。
    _select_subscription: Subscription,
}
impl Console {
    /// 创建页面并立即发起首次加载。
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let monitor = Monitor::new();
        let ticker = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(1))
                    .await;

                // 每秒同步一次内核状态：内核可能自行退出，状态变化后界面跟随。
                let Some((services, running)) = this
                    .update(cx, |console, cx| {
                        let changed = console.sync_kernel();
                        if !console.kernel.running() {
                            // 内核停止后没有数据可读，清掉上一轮曲线避免下次启动残留
                            console.monitor.reset();
                        }
                        if changed {
                            // 托盘菜单只在弹出时读状态，跟着这里一起刷新即可
                            crate::tray::refresh(cx);
                            cx.notify();
                        }
                        (services::ready(cx), console.kernel.running())
                    })
                    .ok()
                else {
                    return;
                };

                if !running {
                    continue;
                }

                // 采样跑在数据层的运行时上，界面线程只等结果，不阻塞渲染。
                let status = match services {
                    Some(services) => {
                        let loader = services.clone();
                        match services
                            .spawn(async move { loader.kernel_status().await })
                            .await
                        {
                            Ok(Ok(status)) => status,
                            Ok(Err(error)) => {
                                log::warn!("[控制台] 读取内核状态失败: {error:#}");
                                None
                            }
                            Err(error) => {
                                log::warn!("[控制台] 读取内核状态任务异常: {error}");
                                None
                            }
                        }
                    }
                    None => None,
                };

                let Some(status) = status else {
                    continue;
                };

                if this
                    .update(cx, |console, cx| {
                        console
                            .monitor
                            .push(status.upload, status.download, status.memory);
                        cx.notify();
                    })
                    .is_err()
                {
                    return;
                }
            }
        });

        let select = toolbar::create_state(window, cx);
        let select_subscription = toolbar::subscribe(&select, cx);

        let mut console = Self {
            load: LoadState::Loading,
            kernel: Kernel::new(services::kernel_data()),
            select,
            selected: None,
            refreshing: false,
            sections: MonitorSection::ALL
                .iter()
                .map(|(_, section)| *section)
                .collect(),
            monitor,
            _ticker: ticker,
            _select_subscription: select_subscription,
        };
        console.reload(window, cx);
        console
    }

    /// 重新读取配置与订阅。
    pub fn reload(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

        self.refreshing = true;
        cx.notify();

        let loader = services.clone();
        let runner = services.clone();
        cx.spawn_in(window, async move |this, cx| {
            let data = loader.spawn(async move { runner.console_data().await });

            let result = match data.await {
                Ok(Ok(data)) => Ok(data),
                Ok(Err(error)) => Err(format!("{error:#}")),
                Err(error) => Err(format!("读取任务异常: {error}")),
            };
            this.update_in(cx, |console, window, cx| {
                console.refreshing = false;
                console.load = match result {
                    Ok(data) => {
                        // 刷新时保留仍然有效的当前选择；否则用上次使用的配置，再退回第一个。
                        let current = console
                            .selected
                            .filter(|id| data.configs.iter().any(|config| config.id == *id));
                        let fallback = data.configs.first().map(|config| config.id);
                        console.selected = current
                            .or_else(|| {
                                data.last_used
                                    .filter(|id| data.configs.iter().any(|config| config.id == *id))
                            })
                            .or(fallback);

                        let options = data
                            .configs
                            .iter()
                            .map(|config| ConfigOption::new(config.id, config.name.clone()))
                            .collect::<Vec<_>>();
                        let selected = console.selected;
                        toolbar::replace(&console.select, options, selected, window, cx);

                        // 预选出来的配置同样要落库，否则托盘菜单一直不知道能启动哪份配置。
                        if let Some(id) = console.selected
                            && data.last_used != Some(id)
                        {
                            console.remember(id, cx);
                        }

                        LoadState::Ready(data)
                    }
                    Err(reason) => LoadState::Failed(reason.into()),
                };
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// 启停内核。
    ///
    /// 启动需要等内核 API 探测通过，按钮在整个等待期间保持 loading；
    /// 加载态由内核实例统一维护，因此托盘或自动运行发起的启动同样会显示。
    pub fn toggle_kernel(&mut self, cx: &mut Context<Self>) {
        if self.kernel.loading {
            return;
        }

        let running = self.kernel.running();
        if !running && self.selected.is_none() {
            return;
        }

        let services = match services::get(cx) {
            Some(AppServices::Ready(services)) => services.clone(),
            Some(AppServices::Failed(_)) | None => return,
        };

        // 先置位再渲染：派生任务要等下一帧才真正开始，期间按钮不能看起来空闲
        self.kernel.loading = true;
        cx.notify();

        let id = self.selected;
        let loader = services.clone();
        let runner = services.clone();
        cx.spawn(async move |this, cx| {
            let task = loader.spawn(async move {
                if running {
                    runner.stop_kernel().await
                } else {
                    match id {
                        Some(id) => runner.start_kernel(id).await,
                        None => Ok(()),
                    }
                }
            });

            let result = match task.await {
                Ok(result) => result.map_err(|error| format!("{error:#}")),
                Err(error) => Err(format!("内核任务异常: {error}")),
            };

            this.update(cx, |console, cx| {
                console.kernel.loading = false;
                console.sync_kernel();
                // 内核实例未记录原因时（例如未初始化）用本次操作的错误补上
                if let Err(reason) = result
                    && console.kernel.reason.is_none()
                {
                    console.kernel.reason = Some(reason);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// 记录选中的配置 id。
    ///
    /// 下次启动用它预选配置，托盘菜单也用它作为「启动内核」的目标。
    /// 写库失败只记录日志：选择本身已经在界面上生效，不影响本次使用。
    pub fn remember(&self, id: i64, cx: &mut Context<Self>) {
        let Some(AppServices::Ready(services)) = services::get(cx) else {
            return;
        };
        let services = services.clone();

        cx.spawn(async move |_, _| {
            let runner = services.clone();
            let task = services.spawn(async move { runner.select_config(id).await });
            if let Ok(Err(error)) = task.await {
                log::warn!("[控制台] 记录选中配置失败: {error:#}");
            }
        })
        .detach();
    }

    /// 同步内核状态，返回状态是否发生变化。
    fn sync_kernel(&mut self) -> bool {
        let data = services::kernel_data();
        let changed = self.kernel.status != data.status
            || self.kernel.acting != data.acting
            || self.kernel.config_id != data.config_id
            || self.kernel.reason != data.reason;
        self.kernel.status = data.status;
        self.kernel.acting = data.acting;
        self.kernel.config_id = data.config_id;
        self.kernel.reason = data.reason;
        changed
    }

    /// 切换一个监控分区的显示。
    pub fn toggle_section(
        &mut self,
        section: MonitorSection,
        selected: bool,
        cx: &mut Context<Self>,
    ) {
        if selected {
            if !self.sections.contains(&section) {
                self.sections.push(section);
            }
        } else {
            self.sections.retain(|item| *item != section);
        }
        cx.notify();
    }

    /// 当前选中的配置。
    pub fn selected_config(&self) -> Option<&types_core::entity::config::Config> {
        let LoadState::Ready(data) = &self.load else {
            return None;
        };
        let id = self.selected?;
        data.configs.iter().find(|config| config.id == id)
    }

    /// 渲染监控控制栏：一组可独立开关的分区卡片。
    ///
    /// 分区只决定监控区显示哪些卡片，与内核是否运行无关：未运行时监控区渲染占位卡片，
    /// 因此这里始终可切换，不做禁用。
    fn render_monitor_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();

        h_flex()
            .w_full()
            .gap_3()
            .children(MonitorSection::ALL.iter().map(|(label, section)| {
                let selected = self.sections.contains(section);
                let section = *section;
                let entity = entity.clone();

                div().flex_1().min_w_0().h_8().child(
                    ItemCard::new(*label)
                        .selected(selected)
                        .selected_style(ItemCardSelectedStyle::Elevated)
                        .center(div().font_weight(FontWeight::MEDIUM).child(*label))
                        .on_click(move |_, cx| {
                            entity.update(cx, |console, cx| {
                                console.toggle_section(section, !selected, cx);
                            });
                        }),
                )
            }))
    }
}

impl Render for Console {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 失败日志按内容展开，窗口放不下时整页纵向滚动，日志因此可以完整查看。
        v_flex()
            .size_full()
            .min_h_0()
            .gap_4()
            .overflow_y_scrollbar()
            .child(toolbar::render(self, cx))
            .child(preview::render(self, cx))
            .children(failure::render(self, cx))
            .child(self.render_monitor_bar(cx))
            .child(
                div()
                    .w_full()
                    .flex_1()
                    // 页面可滚动后剩余高度可能为零，给图表区一个下限，滚动时曲线仍然可读。
                    .min_h(CHART_MIN_HEIGHT)
                    .child(charts::render(
                        &self.monitor,
                        &self.sections,
                        self.kernel.running(),
                        cx.theme(),
                    )),
            )
    }
}
