//! 内核设置分区：可编辑的监听设置。

use gpui_kit::base::{StyledExt as _, h_flex, v_flex};
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants as _;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::{Disableable as _, WindowExt as _};
use gpui_kit::*;

use types_core::dto::setting::SettingKernel;

use super::shared::{ROW_GAP, card, input_row};
use crate::services::{self, AppServices};

/// 分区标题。
pub const TITLE: &str = "内核设置";

/// 内核设置分区。
///
/// 面板与混合代理的监听地址、端口就地修改，改完由「保存」一次性落库；
/// 只读的内核配置见同目录的 `config` 分区。
pub struct Settings {
    /// 已保存的监听设置，用于判断是否有未保存的改动。
    saved: Listen,
    ui_listen: Entity<InputState>,
    ui_port: Entity<InputState>,
    mixed_listen: Entity<InputState>,
    mixed_port: Entity<InputState>,
    /// 是否正在保存。
    saving: bool,
    /// 输入变化订阅，页面存活期间一直保留。
    _subscriptions: Vec<Subscription>,
}

/// 混合协议与内核面板的监听设置。
#[derive(Clone, PartialEq, Eq)]
struct Listen {
    ui_listen: String,
    ui_port: String,
    mixed_listen: String,
    mixed_port: String,
}

impl Listen {
    /// 由内核设置裁剪出监听字段。
    fn of(setting: &SettingKernel) -> Self {
        Self {
            ui_listen: setting.ui_listen.clone(),
            ui_port: setting.ui_port.to_string(),
            mixed_listen: setting.mixed_listen.clone(),
            mixed_port: setting.mixed_port.to_string(),
        }
    }
}

impl Settings {
    /// 创建分区。
    ///
    /// 输入框先建空值，读到设置后由 `fill` 写入：读取是异步的，
    /// 而输入状态必须在渲染前存在。
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let ui_listen = input(window, cx, "监听地址，例如 127.0.0.1");
        let ui_port = input(window, cx, "1 - 65535");
        let mixed_listen = input(window, cx, "监听地址，例如 127.0.0.1");
        let mixed_port = input(window, cx, "1 - 65535");

        // 输入即改值，保存按钮的可用状态与提示都依赖输入内容，因此变化时要重绘分区。
        let subscriptions = [&ui_listen, &ui_port, &mixed_listen, &mixed_port]
            .into_iter()
            .map(|state| {
                cx.subscribe(state, |_, _, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        cx.notify();
                    }
                })
            })
            .collect();

        Self {
            saved: Listen {
                ui_listen: String::new(),
                ui_port: String::new(),
                mixed_listen: String::new(),
                mixed_port: String::new(),
            },
            ui_listen,
            ui_port,
            mixed_listen,
            mixed_port,
            saving: false,
            _subscriptions: subscriptions,
        }
    }

    /// 把读到的内核设置写入输入框，并记为已保存值。
    pub fn fill(&mut self, setting: &SettingKernel, window: &mut Window, cx: &mut Context<Self>) {
        let saved = Listen::of(setting);
        self.ui_listen.update(cx, |state, cx| {
            state.set_value(saved.ui_listen.clone(), window, cx)
        });
        self.ui_port.update(cx, |state, cx| {
            state.set_value(saved.ui_port.clone(), window, cx)
        });
        self.mixed_listen.update(cx, |state, cx| {
            state.set_value(saved.mixed_listen.clone(), window, cx)
        });
        self.mixed_port.update(cx, |state, cx| {
            state.set_value(saved.mixed_port.clone(), window, cx)
        });
        self.saved = saved;
    }

    /// 保存监听设置。
    pub fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.saving {
            return;
        }

        let listen = self.raw(cx);
        if self.errors(cx).iter().any(Option::is_some) {
            return;
        }
        let (Ok(ui_port), Ok(mixed_port)) = (
            listen.ui_port.parse::<u16>(),
            listen.mixed_port.parse::<u16>(),
        ) else {
            return;
        };

        let services = match services::get(cx) {
            Some(AppServices::Ready(services)) => services.clone(),
            _ => return,
        };

        self.saving = true;
        cx.notify();

        // 任务里只用地址字符串，`listen` 本体留给成功后记录已保存值。
        let (ui_listen, mixed_listen) = (listen.ui_listen.clone(), listen.mixed_listen.clone());
        let loader = services.clone();
        let runner = services.clone();
        cx.spawn_in(window, async move |this, cx| {
            let result = loader
                .spawn(async move {
                    runner
                        .save_kernel_listen(&ui_listen, ui_port, &mixed_listen, mixed_port)
                        .await
                })
                .await;

            this.update_in(cx, |settings, window, cx| {
                settings.saving = false;
                match result {
                    Ok(Ok(())) => {
                        settings.saved = listen;
                        window.push_notification(Notification::success("内核监听设置已保存"), cx);
                    }
                    Ok(Err(error)) => window.push_notification(
                        Notification::error(format!("保存内核监听设置失败: {error:#}")),
                        cx,
                    ),
                    Err(error) => window.push_notification(
                        Notification::error(format!("保存任务异常: {error}")),
                        cx,
                    ),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// 输入框中的监听设置。
    fn raw(&self, cx: &App) -> Listen {
        Listen {
            ui_listen: text(&self.ui_listen, cx),
            ui_port: text(&self.ui_port, cx),
            mixed_listen: text(&self.mixed_listen, cx),
            mixed_port: text(&self.mixed_port, cx),
        }
    }

    /// 校验输入，返回每个字段的错误提示，顺序与界面一致。
    fn errors(&self, cx: &App) -> [Option<&'static str>; 4] {
        let raw = self.raw(cx);

        [
            address_error(&raw.ui_listen),
            port_error(&raw.ui_port),
            address_error(&raw.mixed_listen),
            port_error(&raw.mixed_port),
        ]
    }
}

impl Render for Settings {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let errors = self.errors(cx);
        let dirty = self.raw(cx) != self.saved;
        let invalid = errors.iter().any(Option::is_some);

        let content = v_flex()
            .w_full()
            .gap(ROW_GAP)
            .child(input_row("面板监听", &self.ui_listen, errors[0], cx))
            .child(input_row("面板端口", &self.ui_port, errors[1], cx))
            .child(input_row("混合监听", &self.mixed_listen, errors[2], cx))
            .child(input_row("混合端口", &self.mixed_port, errors[3], cx))
            .child(
                h_flex().w_full().gap_3().child(
                    Button::new("kernel-settings-save")
                        .primary()
                        .label("保存")
                        .loading(self.saving)
                        .disabled(!dirty || invalid)
                        .on_click(cx.listener(|settings, _, window, cx| settings.save(window, cx))),
                ),
            )
            .into_any_element();

        card(TITLE, content)
    }
}

/// 读取输入框的当前取值，去掉首尾空白。
fn text(state: &Entity<InputState>, cx: &App) -> String {
    state.read(cx).value().trim().to_string()
}

/// 校验监听地址。
fn address_error(value: &str) -> Option<&'static str> {
    match value.is_empty() {
        true => Some("请填写监听地址"),
        false => None,
    }
}

/// 校验端口：必须是 1 - 65535 之间的整数。
fn port_error(value: &str) -> Option<&'static str> {
    match value.parse::<u16>() {
        Ok(port) if port > 0 => None,
        _ => Some("请填写 1 - 65535 之间的端口"),
    }
}

/// 创建输入框状态。
fn input(
    window: &mut Window,
    cx: &mut Context<Settings>,
    placeholder: &'static str,
) -> Entity<InputState> {
    cx.new(|cx| InputState::new(window, cx).placeholder(placeholder))
}
