use gpui_kit::assets::IconName;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::Sizable as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants as _;
use gpui_kit::component::empty::{
    Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyMediaVariant, EmptyTitle,
};
use gpui_kit::*;

use lingting_gpui_kit::card::Card;
use lingting_gpui_kit::navigate;

/// 控制台路由。内核只能从控制台启动，因此提示里的按钮直接跳过去。
const CONSOLE_PATH: &str = "/console";

/// 内核未启动时的提示。
///
/// 节点、连接、日志这类只依赖内核 API 的页面在内核未运行时没有数据可读，
/// 统一用这一块提示说明原因并给出跳转到控制台的入口，避免各页面各写一份。
pub struct KernelOffline;

impl KernelOffline {
    /// 渲染提示卡片。占满父容器，与页面正常内容的尺寸一致。
    pub fn render(cx: &App) -> impl IntoElement {
        let theme = cx.theme();

        Card::new().size_full().child(
            Empty::new()
                .header(
                    EmptyHeader::new()
                        .media(
                            EmptyMedia::new()
                                .with_variant(EmptyMediaVariant::Icon)
                                .child(IconName::ServerOff),
                        )
                        .title(EmptyTitle::new().child("内核未启动"))
                        .description(
                            EmptyDescription::new()
                                .child("节点信息来自内核 API，需要先启动内核才能读取。"),
                        ),
                )
                .content(
                    EmptyContent::new().child(
                        Button::new("kernel-offline-console")
                            .icon(IconName::Play)
                            .label("前往控制台启动")
                            .on_click(|_, _, cx| navigate(cx, CONSOLE_PATH)),
                    ),
                )
                .text_color(theme.foreground),
        )
    }
}
