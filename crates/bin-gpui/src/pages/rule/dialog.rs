use gpui_kit::base::{StyledExt as _, v_flex};
use gpui_kit::component::button::ButtonVariants as _;
use gpui_kit::component::button::{Button, ButtonVariant};
use gpui_kit::component::dialog::DialogFooter;
use gpui_kit::component::input::{Input, InputState, Textarea, TextareaState};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, WindowExt as _};
use gpui_kit::*;

use lib_core::url::pick_host;
use lingting_gpui_kit::ex_dialog::ExDialog;
use types_core::dto::rule::RuleUpsertDTO;
use types_core::entity::rule::Rule;

use super::RulePage;

/// 刷新间隔的单位换算：界面用小时，落库用毫秒。
const MILLIS_PER_HOUR: i64 = 60 * 60 * 1000;
/// 新增远程规则的默认刷新间隔，与订阅一致。
const DEFAULT_INTERVAL_HOURS: i64 = 10;
/// 远端规则弹窗宽度，单位为 rem，随界面缩放一起变化。
const DIALOG_WIDTH: Rems = rems(28.);
/// 本地规则弹窗宽度：需要容纳规则内容的编辑器，比远端规则更宽。
const LOCAL_DIALOG_WIDTH: Rems = rems(38.);
/// 规则内容编辑器的高度，单位为 rem。
const CONTENT_HEIGHT: Rems = rems(16.);

/// 本地规则的初始内容示例，说明支持的规则写法。
const CONTENT_EXAMPLE: &str = "\
# DOMAIN,nc.example.org
# DOMAIN-KEYWORD,example
# DOMAIN-SUFFIX,example.org
# DST-PORT,22
# IP-CIDR,172.20.0.0/14
# IP-CIDR6,fd93:0d0b:4e8a:2233::/64
# PROCESS-NAME,lingting-nc
";

/// 表单需要的规则字段。
///
/// 实体本身不实现 `Clone`，弹窗又需要在页面可变借用之外持有这些值，
/// 因此这里只取出编辑所需的几个字段。
pub struct RuleDraft {
    id: Option<String>,
    name: String,
    url: String,
    content: String,
    interval: i64,
}

impl RuleDraft {
    /// 由实体裁剪出编辑所需的字段。
    pub fn of(rule: &Rule) -> Self {
        Self {
            id: Some(rule.id.clone()),
            name: rule.name.clone(),
            url: rule.url.clone(),
            content: rule.content.clone(),
            interval: rule.interval,
        }
    }

    /// 新增规则的默认值。
    fn new() -> Self {
        Self {
            id: None,
            name: String::new(),
            url: String::new(),
            content: String::new(),
            interval: 0,
        }
    }

    /// 是否为远端规则，地址非空即远端。
    pub fn is_remote(&self) -> bool {
        !self.url.is_empty()
    }

    /// 界面展示的刷新间隔，单位为小时。
    fn hours(&self) -> i64 {
        if self.interval > 0 {
            self.interval / MILLIS_PER_HOUR
        } else {
            DEFAULT_INTERVAL_HOURS
        }
    }

    /// 界面展示的规则内容，空内容用示例兜底，便于直接编辑。
    fn content(&self) -> String {
        if self.content.is_empty() {
            CONTENT_EXAMPLE.to_string()
        } else {
            self.content.clone()
        }
    }
}

/// 打开新增或编辑弹窗。`draft` 为 `None` 表示新增，`remote` 决定表单形态。
///
/// 远端规则填写地址与刷新间隔；本地规则直接编辑规则内容。
/// 内容与页脚闭包会在弹层重建时再次执行，因此输入框状态必须在打开之前创建并捕获。
pub fn open(
    page: &mut RulePage,
    draft: Option<RuleDraft>,
    remote: bool,
    window: &mut Window,
    cx: &mut Context<RulePage>,
) {
    let draft = draft.unwrap_or_else(RuleDraft::new);
    let title = match draft.id.is_some() {
        true => "编辑规则",
        false => "新增规则",
    };

    // 输入状态在打开前创建：弹层重建时会重新执行内容与页脚闭包，在其中创建会丢失编辑状态。
    let name = cx.new(|cx| {
        let placeholder = if remote {
            "留空时使用地址域名"
        } else {
            "例如：公司内网规则"
        };
        InputState::new(window, cx)
            .placeholder(placeholder)
            .default_value(draft.name.clone())
    });
    let url = cx.new(|cx| {
        InputState::new(window, cx)
            .placeholder("https://example.com/rule.list")
            .default_value(draft.url.clone())
    });
    let interval = cx.new(|cx| {
        InputState::new(window, cx)
            .placeholder("0 表示不自动刷新")
            .default_value(draft.hours().to_string())
    });
    let content = cx.new(|cx| {
        TextareaState::new(window, cx)
            .placeholder("每行一条规则：类型,值")
            .default_value(draft.content())
    });

    let page = cx.entity();
    let id = draft.id;
    window.open_dialog(cx, move |dialog, window, cx| {
        let dialog = ExDialog::from(dialog);
        // 远端规则以地址为准：解析不出域名时保存按钮保持禁用，名称也因此总有兜底值；
        // 本地规则没有兜底来源，名称必填。
        let url_value = url.read(cx).value().trim().to_string();
        let name_value = name.read(cx).value().trim().to_string();
        let (can_save, name_value) = if remote {
            let host = pick_host(&url_value);
            let name = match name_value.is_empty() {
                true => host.clone().unwrap_or_default(),
                false => name_value,
            };
            (host.is_some(), name)
        } else {
            (!name_value.is_empty(), name_value)
        };

        let footer = DialogFooter::new()
            .child(
                Button::new("rule-cancel")
                    .label("取消")
                    .on_click(|_, window, cx| window.close_dialog(cx)),
            )
            .child(
                Button::new("rule-submit")
                    .primary()
                    .label("保存")
                    .loading(page.read(cx).is_submitting())
                    .disabled(!can_save)
                    .on_click({
                        let page = page.clone();
                        let id = id.clone();
                        let name = name.clone();
                        let url = url.clone();
                        let interval = interval.clone();
                        let content = content.clone();
                        move |_, window, cx| {
                            // 点击时再取值：闭包是 `Fn`，且此时才是用户最终确认的内容。
                            let dto = RuleUpsertDTO {
                                id: id.clone(),
                                name: name_value.clone(),
                                url: match remote {
                                    true => url.read(cx).value().trim().to_string(),
                                    false => String::new(),
                                },
                                content: (!remote).then(|| content.read(cx).value().to_string()),
                                interval: match remote {
                                    true => {
                                        interval
                                            .read(cx)
                                            .value()
                                            .trim()
                                            .parse::<i64>()
                                            .unwrap_or_default()
                                            .max(0)
                                            * MILLIS_PER_HOUR
                                    }
                                    false => 0,
                                },
                            };
                            page.update(cx, |page, cx| page.submit(dto, window, cx));
                        }
                    }),
            );

        dialog
            .title(title)
            .w(width(remote).to_pixels(window.rem_size()))
            .overlay_closable(false)
            .content({
                let name = name.clone();
                let url = url.clone();
                let interval = interval.clone();
                let content = content.clone();
                move |content_panel, _, cx| {
                    // 地址已填写但解析不出域名时，在字段下方说明保存被禁用的原因。
                    let value = url.read(cx).value();
                    let hint = (!value.trim().is_empty() && pick_host(value.trim()).is_none())
                        .then(|| {
                            div()
                                .text_color(cx.theme().danger)
                                .child("地址需以 http:// 或 https:// 开头")
                                .into_any_element()
                        });

                    let fields = if remote {
                        v_flex()
                            .gap_3()
                            .child(field("名称", &name, None))
                            .child(field("地址（必填）", &url, hint))
                            .child(field("更新间隔（小时）", &interval, None))
                    } else {
                        // 本地规则名称必填：留空时在字段下方说明保存被禁用的原因。
                        let name_hint = name
                            .read(cx)
                            .value()
                            .trim()
                            .is_empty()
                            .then(|| required_hint(cx, "请填写规则名称"));
                        v_flex()
                            .gap_3()
                            .child(field("名称（必填）", &name, name_hint))
                            .child(content_field(&content))
                    };

                    content_panel.pb_2().child(fields)
                }
            })
            .footer(footer)
            .into_dialog()
    });
}

/// 弹窗宽度：本地规则需要容纳编辑器，因此更宽。
fn width(remote: bool) -> Rems {
    if remote {
        DIALOG_WIDTH
    } else {
        LOCAL_DIALOG_WIDTH
    }
}

/// 一行表单字段：标签在上、控件在下，与页面正文共用同一阅读方向。
///
/// `hint` 把校验说明贴在被描述的字段下方，无校验时传 `None`。
fn field(label: &'static str, state: &Entity<InputState>, hint: Option<AnyElement>) -> AnyElement {
    v_flex()
        .gap_1()
        .child(div().child(label))
        .child(Input::new(state).cleanable(true).aria_label(label))
        .children(hint)
        .into_any_element()
}

/// 规则内容字段：多行编辑器，占满弹窗宽度并固定高度。
fn content_field(state: &Entity<TextareaState>) -> AnyElement {
    v_flex()
        .gap_1()
        .child(div().child("规则内容"))
        .child(
            Textarea::new(state)
                .h(CONTENT_HEIGHT)
                .aria_label("规则内容"),
        )
        .into_any_element()
}

/// 必填字段留空时的提示，样式与地址字段的校验提示一致。
fn required_hint(cx: &App, text: &'static str) -> AnyElement {
    div()
        .text_color(cx.theme().danger)
        .child(text)
        .into_any_element()
}

/// 打开删除确认。
///
/// 删除不可撤销，因此用 `AlertDialog` 点明对象与后果，危险样式只放在删除按钮上。
pub fn confirm_delete(
    page: WeakEntity<RulePage>,
    id: String,
    name: String,
    window: &mut Window,
    cx: &mut App,
) {
    window.open_alert_dialog(cx, move |alert, _, _| {
        let page = page.clone();
        let id = id.clone();
        alert
            .confirm()
            .title(format!("删除「{name}」？"))
            .description("规则与其已保存的内容会被一起删除，且无法恢复。")
            .ok_text("删除")
            .ok_variant(ButtonVariant::Danger)
            .cancel_text("取消")
            .on_ok(move |_, window, cx| {
                page.update(cx, |page, cx| page.remove(id.clone(), window, cx))
                    .ok();
                true
            })
    });
}
