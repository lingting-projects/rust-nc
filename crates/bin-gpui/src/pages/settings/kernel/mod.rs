//! 设置页的内核分区。
//!
//! 由两个卡片组成：内核设置承载可编辑的监听项，内核配置只读展示生成配置时的取值。
//! 两者共用一次读取，由本模块统一发起并把数据分发给两个卡片。

mod config;
mod settings;
mod shared;

use gpui_kit::base::{StyledExt as _, v_flex};
use gpui_kit::*;

use config::Info;
use settings::Settings;
use shared::{ROW_GAP, card, message, value_row};

use crate::services::{self, AppServices};

/// 两个卡片之间的间隔，与设置页各分区之间的间隔一致。
const CARD_GAP: Rems = rems(1.);

/// 内核分区。
pub struct Kernel {
    /// 内核设置卡片，监听项由它自己修改与保存。
    settings: Entity<Settings>,
    /// 只读的内核配置，读到设置后填充。
    load: LoadState,
}

/// 只读分区的加载状态。
enum LoadState {
    Loading,
    Ready(Info),
    Failed(SharedString),
}

impl Kernel {
    /// 创建分区并立即发起首次读取。
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut kernel = Self {
            settings: cx.new(|cx| Settings::new(window, cx)),
            load: LoadState::Loading,
        };
        kernel.reload(window, cx);
        kernel
    }

    /// 读取内核设置，同时填充内核设置卡片与只读的内核配置。
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

        self.load = LoadState::Loading;
        cx.notify();

        let settings = self.settings.clone();
        let loader = services.clone();
        let runner = services.clone();
        cx.spawn_in(window, async move |this, cx| {
            let data = loader.spawn(async move { runner.kernel_setting().await });

            let result = match data.await {
                Ok(Ok(setting)) => Ok(setting),
                Ok(Err(error)) => Err(format!("{error:#}")),
                Err(error) => Err(format!("读取任务异常: {error}")),
            };

            this.update_in(cx, |kernel, window, cx| {
                match result {
                    Ok(setting) => {
                        settings.update(cx, |settings, cx| settings.fill(&setting, window, cx));
                        kernel.load = LoadState::Ready(Info::of(&setting));
                    }
                    Err(reason) => kernel.load = LoadState::Failed(reason.into()),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}

impl Render for Kernel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap(CARD_GAP)
            .child(self.settings.clone())
            .child(self.config(cx))
    }
}

impl Kernel {
    /// 只读的内核配置卡片。
    fn config(&self, cx: &App) -> AnyElement {
        let content = match &self.load {
            LoadState::Loading => message("正在读取内核设置…", cx).into_any_element(),
            LoadState::Failed(reason) => message(reason, cx).into_any_element(),
            LoadState::Ready(info) => rows(info, cx),
        };

        card(config::TITLE, content).into_any_element()
    }
}

/// 逐行渲染只读项。
fn rows(info: &Info, cx: &App) -> AnyElement {
    v_flex()
        .w_full()
        .gap(ROW_GAP)
        .children(
            info.rows()
                .map(|(label, value)| value_row(label, value, cx)),
        )
        .into_any_element()
}
