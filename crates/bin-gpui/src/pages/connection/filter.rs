use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::button::{Toggle, ToggleGroup, ToggleVariant, ToggleVariants as _};
use gpui_kit::*;

use super::ConnectionPage;

/// 快速状态筛选项，按界面顺序排列。
///
/// 已关闭的连接由 `lib-singbox` 的订阅累积，页面按状态取其中一段展示。
pub const STATUSES: [(&str, Status); 3] = [
    ("全部", Status::All),
    ("活动", Status::Active),
    ("关闭", Status::Closed),
];

/// 连接状态筛选。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    /// 全部连接
    All,
    /// 仍处于打开状态的连接
    Active,
    /// 已关闭的连接
    Closed,
}

/// 渲染状态筛选：一组 radio 按钮样式的分段控件，默认选中「活动」。
///
/// `ToggleGroup` 会接管组内每个 `Toggle` 的点击并统一在组级回调里上报结果，
/// 因此选中状态由页面持有，点击后按组回传的勾选数组换算出新的筛选值。
///
/// 选中项用主色填充加主色文字标出：分段控件里未选中项只有描边，
/// 只靠默认的浅灰底几乎看不出选中了哪一项。
pub fn render(page: &ConnectionPage, cx: &mut Context<ConnectionPage>) -> impl IntoElement {
    let current = page.status();
    let entity = cx.entity();
    let primary = cx.theme().primary;

    ToggleGroup::new("connection-status")
        .segmented()
        .with_variant(ToggleVariant::Outline)
        .children(STATUSES.iter().map(|(label, status)| {
            let checked = current == *status;
            let toggle = Toggle::new(format!("connection-status:{}", label))
                .label(*label)
                .checked(checked);
            if checked {
                toggle
                    .bg(primary.opacity(0.12))
                    .text_color(primary)
                    .border_color(primary)
                    .font_weight(FontWeight::SEMIBOLD)
            } else {
                toggle
            }
        }))
        .on_click(move |checks, _, cx| {
            // 组回传的是「只把被点击项取反」的结果：点未选中项时，原来的选中项仍为 true。
            // 因此不能取第一个 true，否则从「活动」切到「全部」后，再点「活动」
            // 或「关闭」时第一个 true 仍是「全部」，界面就卡在「全部」回不去。
            // 与当前选中项不一致的那一项才是被点击的项。
            let Some(status) = checks
                .iter()
                .enumerate()
                .find(|(ix, checked)| **checked != (current == STATUSES[*ix].1))
                .map(|(ix, _)| STATUSES[ix].1)
            else {
                return;
            };
            entity.update(cx, |page, cx| page.set_status(status, cx));
        })
}
