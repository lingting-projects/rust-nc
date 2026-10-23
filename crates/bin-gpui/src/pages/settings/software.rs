//! 设置页的软件分区：窗口行为。
//!
//! 两个开关都立即生效，因此用 `Switch` 而不是需要「保存」的表单；
//! 改动写入设置后由关闭处理与启动流程各自读取，界面不需要再通知它们。

use gpui_kit::base::{StyledExt as _, v_flex};
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::notification::Notification;
use gpui_kit::component::switch::Switch;
use gpui_kit::*;

use service_core::holder::settings::SettingsHolder;

use super::shared::{ROW_GAP, card};
use crate::services::{self, AppServices};

/// 分区标题。
pub const TITLE: &str = "窗口行为";

/// 非 Windows 平台的托盘说明。
const UNSUPPORTED: &str = "当前平台不支持托盘";

/// 关闭到托盘的说明。
const TRAY_HINT: &str = "关闭窗口时隐藏到托盘，进程与内核继续运行，从托盘菜单退出";

/// 最小化启动的说明。
const MINIMIZE_HINT: &str = "启动时不显示主窗口，直接从托盘唤出";

/// 软件分区。
pub struct Software {
    /// 关闭窗口时隐藏到托盘。
    tray: bool,
    /// 启动时隐藏主窗口。
    minimize: bool,
    /// 是否有正在进行的保存，期间不接受新的改动。
    saving: bool,
}

impl Software {
    /// 创建分区并读取当前设置。
    pub fn new() -> Self {
        let software = SettingsHolder::software();
        Self {
            tray: software.tray,
            minimize: software.minimize,
            saving: false,
        }
    }

    /// 修改开关并立即保存。
    ///
    /// 保存失败时回退到已保存的设置并提示原因，避免界面显示与落库结果不一致。
    fn change(&mut self, tray: Option<bool>, minimize: Option<bool>, cx: &mut Context<Self>) {
        if self.saving {
            return;
        }

        if let Some(tray) = tray {
            self.tray = tray;
        }
        if let Some(minimize) = minimize {
            self.minimize = minimize;
        }

        let services = match services::get(cx) {
            Some(AppServices::Ready(services)) => services.clone(),
            _ => {
                self.reload();
                cx.notify();
                return;
            }
        };

        self.saving = true;
        cx.notify();

        let (tray, minimize) = (self.tray, self.minimize);
        let runner = services.clone();
        cx.spawn(async move |this, cx| {
            let task =
                services.spawn(async move { runner.save_window_setting(tray, minimize).await });

            let result = match task.await {
                Ok(result) => result,
                Err(error) => Err(anyhow::anyhow!("保存任务异常: {error}")),
            };

            this.update_in(cx, |software, window, cx| {
                software.saving = false;
                if let Err(error) = result {
                    log::error!("[设置] 保存窗口行为失败: {error:#}");
                    software.reload();
                    window.push_notification(
                        Notification::error(format!("保存窗口行为失败: {error:#}")),
                        cx,
                    );
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// 回到已保存的设置。
    fn reload(&mut self) {
        let software = SettingsHolder::software();
        self.tray = software.tray;
        self.minimize = software.minimize;
    }
}

impl Render for Software {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 托盘只在 Windows 上存在，其它平台两个开关都没有意义
        let supported = crate::tray::SUPPORTED;
        let disabled = !supported || self.saving;

        let content = v_flex()
            .w_full()
            .gap(ROW_GAP)
            .child(
                Switch::new("setting-software-tray")
                    .checked(self.tray)
                    .disabled(disabled)
                    .label("关闭时隐藏到托盘")
                    .on_change(cx.listener(|software, checked, _, cx| {
                        software.change(Some(*checked), None, cx)
                    })),
            )
            .child(hint(TRAY_HINT, cx))
            .child(
                Switch::new("setting-software-minimize")
                    .checked(self.minimize)
                    .disabled(disabled)
                    .label("最小化启动")
                    .on_change(cx.listener(|software, checked, _, cx| {
                        software.change(None, Some(*checked), cx)
                    })),
            )
            .child(hint(MINIMIZE_HINT, cx))
            .children((!supported).then(|| hint(UNSUPPORTED, cx)));

        card(TITLE, content.into_any_element())
    }
}

/// 说明文字。
fn hint(text: &'static str, cx: &App) -> AnyElement {
    div()
        .w_full()
        .text_xs()
        .text_color(cx.theme().muted_foreground)
        .child(text)
        .into_any_element()
}
