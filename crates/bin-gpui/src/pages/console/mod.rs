mod charts;
mod monitor;
mod preview;
mod toolbar;

use gpui_kit::base::{StyledExt as _, h_flex, v_flex};
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::select::{SearchableVec, SelectState};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use lingting_gpui_kit::card::{ItemCard, ItemCardSelectedStyle};

use crate::services::{self, AppServices, ConsoleData};
use monitor::Monitor;
use toolbar::ConfigOption;

/// 各区域卡片的内边距。
///
/// 控制栏、预览栏、监控控制栏、监控区的数值卡与图表卡共用，保证同一层的留白一致。
pub(crate) const PADDING: Rems = rems(1.);

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

/// 内核运行状态。
///
/// 内核尚未接入，状态由本地维护；接入后这里换成真实查询结果。
pub struct Kernel {
    /// 是否正在运行。
    pub running: bool,
    /// 是否正在执行启停操作。
    pub loading: bool,
    /// 正在运行的配置 id，未运行时为 `None`。
    pub config_id: Option<String>,
}

impl Kernel {
    fn stopped() -> Self {
        Self {
            running: false,
            loading: false,
            config_id: None,
        }
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
/// 配置与订阅来自 `service-core`，内核状态与实时监控目前是本地模拟。
pub struct Console {
    load: LoadState,
    kernel: Kernel,
    /// 配置下拉框的状态。
    select: Entity<SelectState<SearchableVec<ConfigOption>>>,
    /// 下拉框中选中的配置 id，未选中时为 `None`。
    selected: Option<String>,
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

                // 内核未运行时监控区不渲染，也不必推进采样。
                if this
                    .update(cx, |console, cx| {
                        if !console.kernel.running {
                            return;
                        }
                        console.monitor.tick();
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
            kernel: Kernel::stopped(),
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
                            .clone()
                            .filter(|id| data.configs.iter().any(|config| &config.id == id));
                        let fallback = data.configs.first().map(|config| config.id.clone());
                        console.selected = current
                            .or_else(|| {
                                data.last_used
                                    .clone()
                                    .filter(|id| data.configs.iter().any(|config| &config.id == id))
                            })
                            .or(fallback);

                        let options = data
                            .configs
                            .iter()
                            .map(|config| ConfigOption::new(config.id.clone(), config.name.clone()))
                            .collect::<Vec<_>>();
                        let selected = console.selected.clone();
                        toolbar::replace(&console.select, options, selected.as_deref(), window, cx);

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
    /// 内核尚未接入，这里用一次短暂的等待代替真实启停，让 loading 状态可见；
    /// 接入后把等待换成真实调用即可，页面其余部分不用改。
    pub fn toggle_kernel(&mut self, cx: &mut Context<Self>) {
        if self.kernel.loading {
            return;
        }

        if !self.kernel.running && self.selected.is_none() {
            return;
        }

        self.kernel.loading = true;
        cx.notify();

        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(400))
                .await;

            this.update(cx, |console, cx| {
                console.kernel.loading = false;
                if console.kernel.running {
                    console.kernel.running = false;
                    console.kernel.config_id = None;
                } else {
                    console.kernel.config_id = console.selected.clone();
                    console.kernel.running = true;
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
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
        let id = self.selected.as_ref()?;
        data.configs.iter().find(|config| &config.id == id)
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
                        .center(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .child(*label),
                        )
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
        v_flex()
            .size_full()
            .min_h_0()
            .gap_4()
            .child(toolbar::render(self, cx))
            .child(preview::render(self, cx))
            .child(self.render_monitor_bar(cx))
            .child(div().w_full().min_h_0().flex_1().child(charts::render(
                &self.monitor,
                &self.sections,
                self.kernel.running,
                cx.theme(),
            )))
    }
}
