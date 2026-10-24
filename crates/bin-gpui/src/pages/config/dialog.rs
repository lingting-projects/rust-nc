use gpui_kit::base::{StyledExt as _, h_flex, v_flex};
use gpui_kit::component::button::ButtonVariants as _;
use gpui_kit::component::button::{Button, ButtonVariant};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::combobox::{Combobox, ComboboxState};
use gpui_kit::component::dialog::DialogFooter;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::searchable_list::SearchableListDelegate as _;
use gpui_kit::component::select::{Select, SelectState};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Sizable as _, WindowExt as _};
use gpui_kit::*;

use lingting_gpui_kit::ex_dialog::ExDialog;
use lingting_gpui_kit::region::RegionMultiSelect;

use types_core::dto::config::ConfigUpsertDTO;
use types_core::entity::config::Config;

use super::ConfigPage;
use super::form::{
    ConfigForm, RULE_SLOT_DIRECT, RULE_SLOT_PROXY, RULE_SLOT_REJECT, RuleDelegate,
    SubscribeDelegate, id_value, rule_peers, rule_register, rule_state, subscribe_delegate,
};

/// 刷新间隔的单位换算：界面用小时，落库用毫秒。
const MILLIS_PER_HOUR: i64 = 60 * 60 * 1000;
/// 弹窗宽度，单位为 rem，随界面缩放一起变化。
const DIALOG_WIDTH: Rems = rems(40.);
/// 表单区域的最大高度，超出后滚动，避免弹窗撑出窗口。
const FORM_MAX_HEIGHT: Rems = rems(26.);
/// 一行开关之间的间距。
const SWITCH_GAP: Rems = rems(1.5);

/// 表单需要的配置字段。
///
/// 实体本身不实现 `Clone`，弹窗又需要在页面可变借用之外持有这些值，
/// 因此这里只取出编辑所需的几个字段。
pub struct ConfigDraft {
    id: Option<i64>,
    name: String,
    subscribe_id: Option<i64>,
    interval: i64,
    rule_direct_ids: Vec<i64>,
    rule_proxy_ids: Vec<i64>,
    rule_reject_ids: Vec<i64>,
    include_name_contains: Vec<String>,
    exclude_name_contains: Vec<String>,
    form: ConfigForm,
}

impl ConfigDraft {
    /// 由实体裁剪出编辑所需的字段。
    pub fn of(config: &Config) -> Self {
        Self {
            id: Some(config.id),
            name: config.name.clone(),
            subscribe_id: config.subscribe_id,
            interval: config.interval,
            rule_direct_ids: config.rule_direct_ids.clone(),
            rule_proxy_ids: config.rule_proxy_ids.clone(),
            rule_reject_ids: config.rule_reject_ids.clone(),
            include_name_contains: config.include_name_contains.clone(),
            exclude_name_contains: config.exclude_name_contains.clone(),
            form: ConfigForm::of(config),
        }
    }

    /// 新增配置的默认值。
    fn new() -> Self {
        Self {
            id: None,
            name: String::new(),
            subscribe_id: None,
            interval: 0,
            rule_direct_ids: Vec::new(),
            rule_proxy_ids: Vec::new(),
            rule_reject_ids: Vec::new(),
            include_name_contains: Vec::new(),
            exclude_name_contains: ConfigUpsertDTO::default().exclude_name_contains,
            form: ConfigForm::new(),
        }
    }

    /// 界面展示的刷新间隔，单位为小时。
    fn hours(&self) -> i64 {
        if self.interval > 0 {
            self.interval / MILLIS_PER_HOUR
        } else {
            ConfigUpsertDTO::default().interval / MILLIS_PER_HOUR
        }
    }
}

/// 打开新增或编辑弹窗。`draft` 为 `None` 表示新增。
///
/// 内容与页脚闭包会在弹层重建时再次执行，因此输入框与下拉框的状态必须在打开之前创建并捕获。
/// 名称与订阅必填，关键字按逗号分隔，区域通过区域多选组件维护。
pub fn open(
    page: &mut ConfigPage,
    draft: Option<ConfigDraft>,
    window: &mut Window,
    cx: &mut Context<ConfigPage>,
) {
    let draft = draft.unwrap_or_else(ConfigDraft::new);
    let title = if draft.id.is_some() {
        "编辑配置"
    } else {
        "新增配置"
    };
    let data = page.data();

    // 输入状态在打开前创建：弹层重建时会重新执行内容与页脚闭包，在其中创建会丢失编辑状态。
    let name = cx.new(|cx| {
        InputState::new(window, cx)
            .placeholder("例如：公司网络")
            .default_value(draft.name.clone())
    });
    let interval = cx.new(|cx| {
        InputState::new(window, cx)
            .placeholder("0 表示不自动刷新")
            .default_value(draft.hours().to_string())
    });
    let include_names = cx.new(|cx| {
        InputState::new(window, cx)
            .placeholder("多个关键字用逗号分隔")
            .default_value(joined(&draft.include_name_contains))
    });
    let exclude_names = cx.new(|cx| {
        InputState::new(window, cx)
            .placeholder("多个关键字用逗号分隔")
            .default_value(joined(&draft.exclude_name_contains))
    });

    // 订阅与规则只在下拉框的状态里使用，创建之后不再需要页面的数据。
    let subscribes = data
        .map(|data| data.subscribes.as_slice())
        .unwrap_or_default();
    let rules = data.map(|data| data.rules.as_slice()).unwrap_or_default();
    let subscribe = select_state(
        subscribe_delegate(subscribes),
        draft.subscribe_id,
        window,
        cx,
    );
    // 三个规则框互相登记：同一条规则只能被其中一个框选中。
    let peers = rule_peers();
    let direct = rule_state(
        rules,
        peers.clone(),
        RULE_SLOT_DIRECT,
        &draft.rule_direct_ids,
        window,
        cx,
    );
    let proxy = rule_state(
        rules,
        peers.clone(),
        RULE_SLOT_PROXY,
        &draft.rule_proxy_ids,
        window,
        cx,
    );
    let reject = rule_state(
        rules,
        peers.clone(),
        RULE_SLOT_REJECT,
        &draft.rule_reject_ids,
        window,
        cx,
    );
    rule_register(&peers, RULE_SLOT_DIRECT, &direct);
    rule_register(&peers, RULE_SLOT_PROXY, &proxy);
    rule_register(&peers, RULE_SLOT_REJECT, &reject);

    let form = cx.new(|_| draft.form);
    let page = cx.entity();
    let id = draft.id;

    window.open_dialog(cx, move |dialog, window, cx| {
        let dialog = ExDialog::from(dialog);
        let state = form.read(cx);
        let name_value = name.read(cx).value().trim().to_string();
        let subscribe_value = subscribe.read(cx).selected_value().cloned();
        // 名称与订阅都必填：缺一个就不允许保存。
        let can_save = !name_value.is_empty() && subscribe_value.is_some();
        let hint = match (name_value.is_empty(), subscribe_value.is_some()) {
            (true, _) => Some("请填写配置名称"),
            (false, false) => Some("请选择订阅"),
            _ => None,
        };

        let footer = DialogFooter::new()
            .child(
                Button::new("config-cancel")
                    .label("取消")
                    .on_click(|_, window, cx| window.close_dialog(cx)),
            )
            .child(
                Button::new("config-submit")
                    .primary()
                    .label("保存")
                    .loading(page.read(cx).is_submitting())
                    .disabled(!can_save)
                    .on_click({
                        let page = page.clone();
                        let id = id;
                        let name = name.clone();
                        let interval = interval.clone();
                        let include_names = include_names.clone();
                        let exclude_names = exclude_names.clone();
                        let subscribe = subscribe.clone();
                        let direct = direct.clone();
                        let proxy = proxy.clone();
                        let reject = reject.clone();
                        let form = form.clone();
                        move |_, window, cx| {
                            // 点击时再取值：闭包是 `Fn`，且此时才是用户最终确认的内容。
                            let state = form.read(cx);
                            let dto = ConfigUpsertDTO {
                                id: id,
                                name: name.read(cx).value().trim().to_string(),
                                tun: state.tun,
                                fake_ip: state.fake_ip,
                                ipv6: state.ipv6,
                                subscribe_id: subscribe
                                    .read(cx)
                                    .selected_value()
                                    .map(|value| value.parse().unwrap_or_default()),
                                geo_cn: state.geo_cn,
                                rule_direct_ids: values(&direct, cx),
                                rule_proxy_ids: values(&proxy, cx),
                                rule_reject_ids: values(&reject, cx),
                                include_area_non: state.include_area_non,
                                include_area: state.include_area.clone(),
                                include_name_contains: split(&include_names.read(cx).value()),
                                exclude_area: state.exclude_area.clone(),
                                exclude_name_contains: split(&exclude_names.read(cx).value()),
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
                let interval = interval.clone();
                let include_names = include_names.clone();
                let exclude_names = exclude_names.clone();
                let subscribe = subscribe.clone();
                let direct = direct.clone();
                let proxy = proxy.clone();
                let reject = reject.clone();
                let form = form.clone();
                move |content, _, cx| {
                    // 外层只按最大高度裁切，内层拿到确定高度后才有滚动范围。
                    let fields = v_flex()
                        .size_full()
                        .gap_3()
                        .overflow_y_scrollbar()
                        .child(field("名称（必填）", &name, hint.map(str::to_string), cx))
                        .child(
                            v_flex().gap_1().child(div().child("订阅（必填）")).child(
                                Select::new(&subscribe)
                                    .id("config-subscribe")
                                    .placeholder("选择订阅")
                                    .accessibility_label("订阅"),
                            ),
                        )
                        .child(switches(&form, cx))
                        .child(areas(&form, cx))
                        .child(
                            v_flex().gap_1().child(div().child("包含名称关键字")).child(
                                Input::new(&include_names)
                                    .cleanable(true)
                                    .aria_label("包含名称关键字"),
                            ),
                        )
                        .child(
                            v_flex().gap_1().child(div().child("排除名称关键字")).child(
                                Input::new(&exclude_names)
                                    .cleanable(true)
                                    .aria_label("排除名称关键字"),
                            ),
                        )
                        .child(rule_field("直连规则", &direct))
                        .child(rule_field("代理规则", &proxy))
                        .child(rule_field("拒绝规则", &reject))
                        .child(field("更新间隔（小时）", &interval, None, cx));

                    content.pb_2().child(
                        v_flex()
                            .max_h(FORM_MAX_HEIGHT)
                            .min_h_0()
                            .child(v_flex().flex_1().min_h_0().overflow_hidden().child(fields)),
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
fn field(
    label: &'static str,
    state: &Entity<InputState>,
    hint: Option<String>,
    cx: &App,
) -> AnyElement {
    let hint = hint.map(|hint| {
        div()
            .text_color(cx.theme().danger)
            .child(hint)
            .into_any_element()
    });

    v_flex()
        .gap_1()
        .child(div().child(label))
        .child(Input::new(state).cleanable(true).aria_label(label))
        .children(hint)
        .into_any_element()
}

/// 开关行：四个布尔字段，随表单一起提交。
fn switches(form: &Entity<ConfigForm>, cx: &mut App) -> AnyElement {
    let state = form.read(cx);

    h_flex()
        .gap(SWITCH_GAP)
        .child(toggle("config-tun", "TUN", state.tun, form, |form| {
            &mut form.tun
        }))
        .child(toggle("config-ipv6", "IPv6", state.ipv6, form, |form| {
            &mut form.ipv6
        }))
        .child(toggle(
            "config-fake-ip",
            "FakeIP",
            state.fake_ip,
            form,
            |form| &mut form.fake_ip,
        ))
        .child(toggle(
            "config-geo-cn",
            "GeoCn",
            state.geo_cn,
            form,
            |form| &mut form.geo_cn,
        ))
        .into_any_element()
}

/// 单个勾选项：回调把新值写回表单，界面按受控值重新渲染。
///
/// 表单要点保存才生效，因此用 `Checkbox` 而不是立即生效的 `Switch`。
fn toggle(
    id: &'static str,
    label: &'static str,
    checked: bool,
    form: &Entity<ConfigForm>,
    setter: fn(&mut ConfigForm) -> &mut bool,
) -> AnyElement {
    let form = form.clone();
    Checkbox::new(id)
        .small()
        .checked(checked)
        .label(label)
        .on_change(move |value, _, cx| {
            form.update(cx, |form, cx| {
                *setter(form) = *value;
                cx.notify();
            });
        })
        .into_any_element()
}

/// 区域行：无区域节点开关、包含区域与排除区域的多选。
fn areas(form: &Entity<ConfigForm>, cx: &mut App) -> AnyElement {
    let state = form.read(cx);
    let include = state.include_area.clone();
    let exclude = state.exclude_area.clone();
    let non_area = state.include_area_non;

    v_flex()
        .gap_3()
        .child(toggle(
            "config-include-area-non",
            "包含无区域节点",
            non_area,
            form,
            |form| &mut form.include_area_non,
        ))
        .child(
            v_flex().gap_1().child(div().child("包含区域")).child(
                // 区域多选：值变化直接写回表单，渲染时按受控值同步。
                RegionMultiSelect::new("config-include-area")
                    .value(include)
                    .placeholder("全部区域")
                    .search_placeholder("搜索区域")
                    .on_change({
                        let form = form.clone();
                        move |values, _, cx| {
                            let values = values.iter().map(|iso| iso.to_string()).collect();
                            form.update(cx, |form, cx| {
                                form.include_area = values;
                                cx.notify();
                            });
                        }
                    }),
            ),
        )
        .child(
            v_flex().gap_1().child(div().child("排除区域")).child(
                RegionMultiSelect::new("config-exclude-area")
                    .value(exclude)
                    .on_change({
                        let form = form.clone();
                        move |values, _, cx| {
                            let values = values.iter().map(|iso| iso.to_string()).collect();
                            form.update(cx, |form, cx| {
                                form.exclude_area = values;
                                cx.notify();
                            });
                        }
                    }),
            ),
        )
        .into_any_element()
}

/// 规则字段：多选下拉，已选规则在弹窗内即时生效。
fn rule_field(label: &'static str, state: &Entity<ComboboxState<RuleDelegate>>) -> AnyElement {
    v_flex()
        .gap_1()
        .child(div().child(label))
        .child(
            Combobox::new(state)
                .placeholder("选择规则")
                .search_placeholder("搜索规则")
                .cleanable(true),
        )
        .into_any_element()
}

/// 建立订阅下拉的状态，按 id 定位初始选中项。
fn select_state(
    delegate: SubscribeDelegate,
    selected: Option<i64>,
    window: &mut Window,
    cx: &mut App,
) -> Entity<SelectState<SubscribeDelegate>> {
    let index = selected.and_then(|selected| delegate.position(&id_value(selected)));
    cx.new(|cx| SelectState::new(delegate, index, window, cx).searchable(true))
}

/// 读取多选下拉的已选值。
fn values(state: &Entity<ComboboxState<RuleDelegate>>, cx: &App) -> Vec<i64> {
    state
        .read(cx)
        .selected_values()
        .into_iter()
        .map(|value| value.parse().unwrap_or_default())
        .collect()
}

/// 关键字输入框的展示文本。
fn joined(values: &[String]) -> String {
    values.join(", ")
}

/// 按逗号拆分关键字，忽略空项与首尾空白。
fn split(value: &str) -> Vec<String> {
    value
        .split([',', '，'])
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

/// 打开删除确认。
///
/// 删除不可撤销，因此用 `AlertDialog` 点明对象与后果，危险样式只放在删除按钮上。
pub fn confirm_delete(
    page: WeakEntity<ConfigPage>,
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
            .description("配置会被删除，且无法恢复。")
            .ok_text("删除")
            .ok_variant(ButtonVariant::Danger)
            .cancel_text("取消")
            .on_ok(move |_, window, cx| {
                page.update(cx, |page, cx| page.remove(id, window, cx)).ok();
                true
            })
    });
}
