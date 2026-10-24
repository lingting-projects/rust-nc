use gpui_kit::base::{StyledExt as _, v_flex};
use gpui_kit::component::button::ButtonVariants as _;
use gpui_kit::component::button::{Button, ButtonVariant};
use gpui_kit::component::dialog::DialogFooter;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, WindowExt as _};
use gpui_kit::*;

use lingting_gpui_kit::ex_dialog::ExDialog;

use lib_core::url::pick_host;
use types_core::dto::subscribe::SubscribeUpsertDTO;
use types_core::entity::subscribe::Subscribe;

use super::SubscribePage;

/// 刷新间隔的单位换算：界面用小时，落库用毫秒。
const MILLIS_PER_HOUR: i64 = 60 * 60 * 1000;
/// 新增订阅的默认刷新间隔，与旧界面一致。
const DEFAULT_INTERVAL_HOURS: i64 = 10;
/// 弹窗宽度，单位为 rem，随界面缩放一起变化。
const DIALOG_WIDTH: Rems = rems(28.);

/// 表单需要的订阅字段。
///
/// 实体本身不实现 `Clone`，弹窗又需要在页面可变借用之外持有这些值，
/// 因此这里只取出编辑所需的几个字段。
pub struct SubscribeDraft {
    id: Option<i64>,
    name: String,
    url: String,
    interval: i64,
}

impl SubscribeDraft {
    /// 由实体裁剪出编辑所需的字段。
    pub fn of(subscribe: &Subscribe) -> Self {
        Self {
            id: Some(subscribe.id),
            name: subscribe.name.clone(),
            url: subscribe.url.clone(),
            interval: subscribe.interval,
        }
    }

    /// 新增订阅的默认值。
    fn new() -> Self {
        Self {
            id: None,
            name: String::new(),
            url: String::new(),
            interval: 0,
        }
    }

    /// 界面展示的刷新间隔，单位为小时。
    fn hours(&self) -> i64 {
        if self.interval > 0 {
            self.interval / MILLIS_PER_HOUR
        } else {
            DEFAULT_INTERVAL_HOURS
        }
    }
}

/// 打开新增或编辑弹窗。`draft` 为 `None` 表示新增。
///
/// 表单字段是订阅本身的名称、地址与刷新间隔，提交后由页面负责写库与刷新列表。
/// 地址必填，且必须能解析出域名；名称可留空，保存时用地址的域名兜底。
/// 内容与页脚闭包会在弹层重建时再次执行，因此输入框状态必须在打开之前创建并捕获。
pub fn open(
    page: &mut SubscribePage,
    draft: Option<SubscribeDraft>,
    window: &mut Window,
    cx: &mut Context<SubscribePage>,
) {
    let draft = draft.unwrap_or_else(SubscribeDraft::new);
    let title = if draft.id.is_some() {
        "编辑订阅"
    } else {
        "新增订阅"
    };

    // 输入状态在打开前创建：弹层重建时会重新执行内容与页脚闭包，在其中创建会丢失编辑状态。
    let name = cx.new(|cx| {
        InputState::new(window, cx)
            .placeholder("留空时使用地址域名")
            .default_value(draft.name.clone())
    });
    let url = cx.new(|cx| {
        InputState::new(window, cx)
            .placeholder("https://example.com/subscribe")
            .default_value(draft.url.clone())
    });
    let interval = cx.new(|cx| {
        InputState::new(window, cx)
            .placeholder("0 表示不自动刷新")
            .default_value(draft.hours().to_string())
    });

    let page = cx.entity();
    let id = draft.id;
    window.open_dialog(cx, move |dialog, window, cx| {
        let dialog = ExDialog::from(dialog);
        // 地址必填：解析不出域名时保存按钮保持禁用，名称也因此总有兜底值。
        let url_value = url.read(cx).value().to_string();
        let can_save = pick_host(url_value.trim()).is_some();
        let name_value = {
            let name = name.read(cx).value().trim().to_string();
            match name.is_empty() {
                true => pick_host(url_value.trim()).unwrap_or_default(),
                false => name,
            }
        };
        let footer = DialogFooter::new()
            .child(
                Button::new("subscribe-cancel")
                    .label("取消")
                    .on_click(|_, window, cx| window.close_dialog(cx)),
            )
            .child(
                Button::new("subscribe-submit")
                    .primary()
                    .label("保存")
                    .loading(page.read(cx).is_submitting())
                    .disabled(!can_save)
                    .on_click({
                        let page = page.clone();
                        let id = id;
                        let name = name.clone();
                        let url = url.clone();
                        let interval = interval.clone();
                        move |_, window, cx| {
                            // 点击时再取值：闭包是 `Fn`，且此时才是用户最终确认的内容。
                            let dto = SubscribeUpsertDTO {
                                id: id,
                                name: name_value.clone(),
                                url: url_value.clone(),
                                content: None,
                                interval: interval
                                    .read(cx)
                                    .value()
                                    .trim()
                                    .parse::<i64>()
                                    .unwrap_or_default()
                                    .max(0)
                                    * MILLIS_PER_HOUR,
                            };
                            page.update(cx, |page, cx| page.submit(dto, window, cx));
                        }
                    }),
            );

        dialog
            .title(title)
            .w(DIALOG_WIDTH.to_pixels(window.rem_size()))
            .overlay_closable(false)
            .content({
                let name = name.clone();
                let url = url.clone();
                let interval = interval.clone();
                move |content, _, cx| {
                    // 地址已填写但解析不出域名时，在字段下方说明保存被禁用的原因。
                    let value = url.read(cx).value();
                    let hint = (!value.trim().is_empty() && pick_host(value.trim()).is_none())
                        .then(|| {
                            div()
                                .text_color(cx.theme().danger)
                                .child("地址需以 http:// 或 https:// 开头")
                                .into_any_element()
                        });

                    content.pb_2().child(
                        v_flex()
                            .gap_3()
                            .child(field("名称", &name, None))
                            .child(field("地址（必填）", &url, hint))
                            .child(field("更新间隔（小时）", &interval, None)),
                    )
                }
            })
            .footer(footer)
            .into_dialog()
    });
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

/// 打开删除确认。
///
/// 删除不可撤销，因此用 `AlertDialog` 点明对象与后果，危险样式只放在删除按钮上。
pub fn confirm_delete(
    page: WeakEntity<SubscribePage>,
    id: i64,
    name: String,
    window: &mut Window,
    cx: &mut App,
) {
    window.open_alert_dialog(cx, move |alert, _, _| {
        let page = page.clone();
        let id = id;
        alert
            .confirm()
            .title(format!("删除「{name}」？"))
            .description("订阅与其已保存的节点会被一起删除，且无法恢复。")
            .ok_text("删除")
            .ok_variant(ButtonVariant::Danger)
            .cancel_text("取消")
            .on_ok(move |_, window, cx| {
                page.update(cx, |page, cx| page.remove(id, window, cx)).ok();
                true
            })
    });
}
