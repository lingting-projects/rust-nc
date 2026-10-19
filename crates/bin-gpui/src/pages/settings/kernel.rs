use gpui_kit::base::{StyledExt as _, h_flex, v_flex};
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants as _;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::{ActiveTheme as _, Disableable as _, WindowExt as _};
use gpui_kit::*;

use lib_core::constants;
use lingting_gpui_kit::card::Card;
use types_core::dto::setting::SettingKernel;

use crate::services::{self, AppServices};

/// 卡片内边距，与其它页面保持一致。
const PADDING: Rems = rems(1.);

/// 标签列宽度：固定后各行内容从同一条起始线开始。
const LABEL_WIDTH: Rems = rems(5.);

/// 标签与内容之间的间隔。
const COLUMN_GAP: Rems = rems(1.);

/// 各行之间的间隔。
const ROW_GAP: Rems = rems(0.75);

/// 分区标题。
const TITLE: &str = "内核";

/// 只读项的占位，表示该项没有可用取值。
const EMPTY: &str = "未配置";

/// 内核设置分区。
///
/// 只读项（sing-box 版本、延迟测试地址、两个分流 DNS）直接展示；
/// 监听设置就地修改，改完由「保存」一次性落库。
pub struct Kernel {
    load: LoadState,
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

/// 分区的加载状态。
enum LoadState {
    Loading,
    Ready(Info),
    Failed(SharedString),
}

/// 只读展示项。
struct Info {
    /// 随包分发的 sing-box 版本。
    version: &'static str,
    /// 连通性测试地址。
    test_url: &'static str,
    /// 国内分流 DNS，仅展示内部数组的第一个。
    dns_cn: String,
    /// 代理分流 DNS，仅展示内部数组的第一个。
    dns_proxy: String,
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

impl Kernel {
    /// 创建分区并立即发起首次读取。
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // 输入框先建空值，读到设置后再写入：读取是异步的，而输入状态必须在渲染前存在。
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

        let mut kernel = Self {
            load: LoadState::Loading,
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
        };
        kernel.reload(window, cx);
        kernel
    }

    /// 读取内核设置。
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
                        let saved = Listen::of(&setting);
                        kernel.ui_listen.update(cx, |state, cx| {
                            state.set_value(saved.ui_listen.clone(), window, cx)
                        });
                        kernel.ui_port.update(cx, |state, cx| {
                            state.set_value(saved.ui_port.clone(), window, cx)
                        });
                        kernel.mixed_listen.update(cx, |state, cx| {
                            state.set_value(saved.mixed_listen.clone(), window, cx)
                        });
                        kernel.mixed_port.update(cx, |state, cx| {
                            state.set_value(saved.mixed_port.clone(), window, cx)
                        });
                        kernel.saved = saved;
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

            this.update_in(cx, |kernel, window, cx| {
                kernel.saving = false;
                match result {
                    Ok(Ok(())) => {
                        kernel.saved = listen;
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

impl Info {
    /// 由内核设置取出只读项。
    ///
    /// 内核只支持一个 DNS 配置，因此每个分流只展示内部数组的第一个地址。
    fn of(setting: &SettingKernel) -> Self {
        Self {
            version: lib_singbox::VERSION,
            test_url: constants::TEST_URL,
            dns_cn: first(&setting.dns_cn),
            dns_proxy: first(&setting.dns_proxy),
        }
    }
}

impl Render for Kernel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = match &self.load {
            LoadState::Loading => message("正在读取内核设置…", cx).into_any_element(),
            LoadState::Failed(reason) => message(reason, cx).into_any_element(),
            LoadState::Ready(info) => self.ready(info, cx),
        };

        Card::new()
            .w_full()
            .flex_shrink_0()
            .title_padding(PADDING)
            .content_padding(PADDING)
            .title(div().font_weight(FontWeight::MEDIUM).child(TITLE))
            .child(content)
    }
}

impl Kernel {
    /// 分区内容：只读项在上，监听设置在下，末尾一行保存。
    fn ready(&self, info: &Info, cx: &mut Context<Self>) -> AnyElement {
        let errors = self.errors(cx);
        let dirty = self.raw(cx) != self.saved;
        let invalid = errors.iter().any(Option::is_some);

        v_flex()
            .w_full()
            .gap(ROW_GAP)
            .child(value_row("版本", info.version, cx))
            .child(value_row("延迟测试", info.test_url, cx))
            .child(value_row("国内 DNS", &info.dns_cn, cx))
            .child(value_row("代理 DNS", &info.dns_proxy, cx))
            .child(input_row("面板监听", &self.ui_listen, errors[0], cx))
            .child(input_row("面板端口", &self.ui_port, errors[1], cx))
            .child(input_row("混合监听", &self.mixed_listen, errors[2], cx))
            .child(input_row("混合端口", &self.mixed_port, errors[3], cx))
            .child(
                h_flex().w_full().gap_3().child(
                    Button::new("kernel-save")
                        .primary()
                        .label("保存")
                        .loading(self.saving)
                        .disabled(!dirty || invalid)
                        .on_click(cx.listener(|kernel, _, window, cx| kernel.save(window, cx))),
                ),
            )
            .into_any_element()
    }
}

/// 一行只读项：左侧标签，右侧取值。
fn value_row(label: &'static str, value: &str, cx: &App) -> AnyElement {
    let value = match value.is_empty() {
        true => EMPTY,
        false => value,
    };

    h_flex()
        .w_full()
        .items_start()
        .gap(COLUMN_GAP)
        .child(label_cell(label, cx))
        .child(
            // 取值是标识符，用等宽字体与正文区分；`flex_1` 配 `min_w_0` 才能在窄窗口里换行。
            div()
                .flex_1()
                .min_w_0()
                .font_family(cx.theme().mono_font_family.clone())
                .child(value.to_string()),
        )
        .into_any_element()
}

/// 一行可编辑项：左侧标签，右侧输入框，校验说明贴在被描述的输入框下方。
fn input_row(
    label: &'static str,
    state: &Entity<InputState>,
    error: Option<&'static str>,
    cx: &App,
) -> AnyElement {
    let hint = error.map(|text| {
        div()
            .text_color(cx.theme().danger)
            .child(text)
            .into_any_element()
    });

    h_flex()
        .w_full()
        .items_start()
        .gap(COLUMN_GAP)
        .child(label_cell(label, cx))
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap_1()
                .child(Input::new(state).cleanable(true).aria_label(label))
                .children(hint),
        )
        .into_any_element()
}

/// 行首的标签单元格。
fn label_cell(label: &'static str, cx: &App) -> AnyElement {
    div()
        .w(LABEL_WIDTH)
        .flex_shrink_0()
        .text_color(cx.theme().muted_foreground)
        .child(label)
        .into_any_element()
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

/// 取数组第一个元素，空数组用空串表示未配置。
fn first(items: &[String]) -> String {
    items.first().cloned().unwrap_or_default()
}

/// 分区状态提示，失败时用危险色。
fn message(reason: &str, cx: &App) -> impl IntoElement {
    div()
        .w_full()
        .text_color(cx.theme().danger)
        .child(reason.to_string())
}

/// 创建输入框状态。
fn input(
    window: &mut Window,
    cx: &mut Context<Kernel>,
    placeholder: &'static str,
) -> Entity<InputState> {
    cx.new(|cx| InputState::new(window, cx).placeholder(placeholder))
}
