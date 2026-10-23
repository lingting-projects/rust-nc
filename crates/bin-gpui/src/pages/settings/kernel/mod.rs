//! 设置页的内核分区。
//!
//! 由两个卡片组成：内核设置承载可编辑的监听项，内核配置只读展示生成配置时的取值。
//! 两者共用一次读取，由本模块统一发起并把数据分发给两个卡片。

mod config;
mod settings;

use gpui_kit::base::{StyledExt as _, v_flex};
use gpui_kit::*;

use crate::pages::settings::shared::{ROW_GAP, card, message, value_row};
use config::Info;
use service_core::holder::settings::SettingsHolder;
use settings::Settings;

use crate::services::{self, AppServices};

/// 两个卡片之间的间隔，与设置页各分区之间的间隔一致。
const CARD_GAP: Rems = rems(1.);

/// 内核分区。
pub struct Kernel {
    /// 内核设置卡片，监听项由它自己修改与保存。
    settings: Entity<Settings>,
    /// 只读的内核配置，读取失败时保留失败原因。
    load: Result<Info, SharedString>,
}

impl Kernel {
    /// 创建分区并立即读取内核设置。
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut kernel = Self {
            settings: cx.new(|cx| Settings::new(window, cx)),
            load: Err("数据层未初始化".into()),
        };
        kernel.reload(window, cx);
        kernel
    }

    /// 读取内核设置，同时填充内核设置卡片与只读的内核配置。
    ///
    /// 设置由 [`SettingsHolder`] 保存在内存中，读取不需要等待数据库。
    pub fn reload(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let setting = match services::get(cx) {
            Some(AppServices::Ready(_)) => SettingsHolder::kernel(),
            Some(AppServices::Failed(reason)) => {
                self.load = Err(reason.clone());
                return;
            }
            None => {
                self.load = Err("数据层未初始化".into());
                return;
            }
        };

        self.settings
            .update(cx, |settings, cx| settings.fill(&setting, window, cx));
        self.load = Ok(Info::of(&setting));
        cx.notify();
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
            Ok(info) => rows(info, cx),
            Err(reason) => message(reason, cx).into_any_element(),
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
