use gpui_kit::base::{StyledExt as _, h_flex};
use gpui_kit::component::button::Button;
use gpui_kit::component::description_list::DescriptionList;
use gpui_kit::component::{Sizable as _, WindowExt as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use lingting_gpui_kit::ex_dialog::ExDialog;

use lib_singbox::ConnectionRecord;

use super::rows;
use crate::utils::format;

/// 详情弹窗宽度，单位为 rem，随界面缩放一起变化。
const DIALOG_WIDTH: Rems = rems(32.);

/// 打开连接详情弹窗。
///
/// 连接会随内核推送持续变化，因此弹窗打开时把整行数据拷贝进去，
/// 展示的是点击那一刻的快照，不会因为列表刷新而中途改内容。
pub fn open(row: ConnectionRecord, window: &mut Window, cx: &mut App) {
    let title = row.info.destination.clone();
    let width = DIALOG_WIDTH.to_pixels(window.rem_size());

    window.open_dialog(cx, move |dialog, _, _| {
        ExDialog::from(dialog)
            .title(title.clone())
            .w(width)
            .content({
                let row = row.clone();
                move |content, _, _| content.child(details(&row))
            })
            .footer(
                h_flex().justify_end().child(
                    Button::new("connection-detail-close")
                        .label("关闭")
                        .on_click(|_, window, cx| window.close_dialog(cx)),
                ),
            )
            .into_dialog()
    });
}

/// 详情正文：两列描述列表，标签在左、值在右。
fn details(row: &ConnectionRecord) -> impl IntoElement {
    let info = &row.info;

    DescriptionList::new()
        .columns(2)
        .small()
        .label_width(rems(6.))
        .item("入口", inbound(row), 1)
        .item("网络", network(&info.network), 1)
        .item("进程", process(&info.process), 2)
        .item("目标", info.destination.clone(), 2)
        .item("规则", text_or_dash(&info.rule), 1)
        .item("链路", rows::chain(row), 1)
        .item("源地址", info.source.clone(), 1)
        .item("建立时间", format::since(info.created_at), 1)
        .item("下载", format::data_size(info.download), 1)
        .item("上传", format::data_size(info.upload), 1)
        .separator()
        .item("连接 ID", info.id.clone(), 2)
}

/// 入口：入站标签与类型，例如 `mixed-in (mixed)`。
fn inbound(row: &ConnectionRecord) -> String {
    let info = &row.info;
    if info.inbound_type.is_empty() {
        return text_or_dash(&info.inbound);
    }
    format!("{} ({})", info.inbound, info.inbound_type)
}

/// 网络类型，内核未给出时展示为 `-`。
fn network(value: &str) -> String {
    text_or_dash(&value.to_uppercase())
}

/// 进程路径，取不到进程信息时说明原因。
fn process(value: &str) -> String {
    if value.is_empty() {
        return "未知（内核未提供进程信息）".to_string();
    }
    value.to_string()
}

/// 空值统一展示为 `-`。
fn text_or_dash(value: &str) -> String {
    if value.is_empty() {
        "-".to_string()
    } else {
        value.to_string()
    }
}
