use gpui_kit::component::Sizable as _;
use gpui_kit::component::button::{Toggle, ToggleGroup, ToggleVariant, ToggleVariants as _};
use gpui_kit::*;

use super::ConnectionPage;

/// 快速状态筛选项，按界面顺序排列。
///
/// 内核只推送仍处于打开状态的连接，因此「关闭」在这里指筛选结果为空，
/// 保留该选项是为了与常见连接列表的口径一致，切换后给出对应的空状态说明。
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
pub fn render(page: &ConnectionPage, cx: &mut Context<ConnectionPage>) -> impl IntoElement {
    let current = page.status();
    let entity = cx.entity();

    ToggleGroup::new("connection-status")
        .segmented()
        .small()
        .with_variant(ToggleVariant::Outline)
        .children(STATUSES.iter().map(|(label, status)| {
            Toggle::new(format!("connection-status:{}", label))
                .label(*label)
                .checked(current == *status)
        }))
        .on_click(move |checks, _, cx| {
            // 分段控件同时只会有一个选中项，取第一个被勾选的位置即可。
            let Some(status) = checks
                .iter()
                .position(|checked| *checked)
                .map(|ix| STATUSES[ix].1)
            else {
                return;
            };
            entity.update(cx, |page, cx| page.set_status(status, cx));
        })
}
