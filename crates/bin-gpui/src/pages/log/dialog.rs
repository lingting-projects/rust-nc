//! 日志详情弹窗。
//!
//! 块在内存里只保留首行前缀，完整内容按字节偏移在后台线程读取。
//! 读取过程中展示进度，关闭弹窗立即中断读取，不再继续占用 IO。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui_kit::base::{StyledExt as _, h_flex, v_flex};
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::progress::Progress;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::{Disableable as _, WindowExt as _};
use gpui_kit::*;

use lingting_gpui_kit::ex_dialog::ExDialog;

use lib_singbox::LogBlock;

use crate::services;
use crate::utils::window::window_state;

/// 详情弹窗宽度。
const DIALOG_WIDTH: Rems = rems(48.);

/// 完整内容的文本区域最大高度，超出后滚动查看。
const CONTENT_HEIGHT: Rems = rems(24.);

/// 读取进度的刷新间隔，避免每读一块就重绘一次。
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

/// 详情内容的加载状态。
enum State {
    Loading {
        read: u64,
        total: u64,
    },
    /// 读到完整内容。
    Ready(String),
    /// 日志文件已被内核重写，偏移失效，只能展示首行。
    Stale,
}

/// 详情弹窗的内容与后台读取。
///
/// 作为独立实体存在，弹窗关闭时随 `Drop` 一起让读取线程停下来。
pub struct Detail {
    block: LogBlock,
    state: State,
    /// 读取是否已被放弃，弹窗关闭时置为 `true`。
    cancelled: Arc<Mutex<bool>>,
}

impl Detail {
    /// 创建详情并启动后台读取。
    pub fn open(block: LogBlock, window: &mut Window, cx: &mut App) {
        let read = Arc::new(AtomicU64::new(0));
        let cancelled = Arc::new(Mutex::new(false));
        let total = block.bytes;

        let detail = cx.new({
            let block = block.clone();
            let read = read.clone();
            let cancelled = cancelled.clone();
            move |cx| {
                start(&block, read.clone(), cancelled.clone(), cx);
                Self {
                    block,
                    state: State::Loading { read: 0, total },
                    cancelled,
                }
            }
        });

        // 进度轮询：只在加载中重绘，读完即退出
        cx.spawn({
            let detail = detail.clone();
            let read = read.clone();
            async move |cx| {
                loop {
                    window_state().tick(cx, PROGRESS_INTERVAL).await;
                    // 读取结束或实体已被释放时停止轮询
                    let alive = detail.update(cx, |detail, cx| {
                        let State::Loading { read: current, .. } = &mut detail.state else {
                            return false;
                        };
                        let value = read.load(Ordering::Relaxed);
                        if value != *current {
                            *current = value;
                            cx.notify();
                        }
                        true
                    });
                    if !alive {
                        return;
                    }
                }
            }
        })
        .detach();

        let width = DIALOG_WIDTH.to_pixels(window.rem_size());
        let title = match block.level.is_known() {
            true => format!("{} 日志", block.level.label()),
            false => "日志".to_string(),
        };

        window.open_dialog(cx, move |dialog, _, _| {
            ExDialog::from(dialog)
                .title(title.clone())
                .w(width)
                .content({
                    let detail = detail.clone();
                    move |content, _, _| content.child(detail.clone())
                })
                .footer(
                    h_flex().justify_end().child(
                        Button::new("log-detail-close")
                            .label("关闭")
                            .on_click(|_, window, cx| window.close_dialog(cx)),
                    ),
                )
                .into_dialog()
        });
    }
}

impl Drop for Detail {
    fn drop(&mut self) {
        // 弹窗关闭即放弃读取，后台线程在下一块处退出
        if let Ok(mut cancelled) = self.cancelled.lock() {
            *cancelled = true;
        }
    }
}

impl Render for Detail {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        match &self.state {
            State::Loading { read, total } => {
                let percent = match total {
                    0 => 0.,
                    total => (*read as f64 / *total as f64 * 100.).min(100.) as f32,
                };

                v_flex()
                    .w_full()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(format!(
                                "正在读取 {} / {}",
                                crate::utils::format::data_size(*read),
                                crate::utils::format::data_size(*total)
                            )),
                    )
                    .child(
                        Progress::new("log-detail-progress")
                            .value(percent)
                            .accessibility_label("日志读取进度"),
                    )
                    .into_any_element()
            }
            State::Ready(content) => body(&self.block, Some(content), cx),
            State::Stale => body(&self.block, None, cx),
        }
    }
}

/// 启动后台读取。
///
/// 读取放在阻塞线程池上，大块日志不会占住界面线程；`progress` 只汇报进度，
/// 返回 `false` 表示弹窗已关闭，读取立即停止。
fn start(
    block: &LogBlock,
    read: Arc<AtomicU64>,
    cancelled: Arc<Mutex<bool>>,
    cx: &mut Context<Detail>,
) {
    let Some(services) = services::ready(cx) else {
        cx.entity().update(cx, |detail, cx| {
            detail.state = State::Stale;
            cx.notify();
        });
        return;
    };

    let block = block.clone();
    cx.spawn(async move |this, cx| {
        // 读取是同步阻塞 IO，放到后台线程池，避免大块日志卡住界面
        let read_handle = read.clone();
        let cancel_handle = cancelled.clone();
        let result = cx
            .background_executor()
            .spawn(async move {
                services.kernel_log_detail(&block, move |size| {
                    read_handle.store(size, Ordering::Relaxed);
                    !*cancel_handle
                        .lock()
                        .unwrap_or_else(|error| error.into_inner())
                })
            })
            .await;

        let state = match result {
            Ok(Some(content)) => State::Ready(content),
            Ok(None) => State::Stale,
            Err(error) => {
                log::warn!("[日志] 读取日志详情失败: {error:#}");
                State::Stale
            }
        };

        this.update(cx, |detail, cx| {
            detail.state = state;
            cx.notify();
        })
        .ok();
    })
    .detach();
}

/// 弹窗正文：完整内容按等宽字体展示，可纵向滚动。
///
/// 正常情况下 `content` 是整块的原始内容；只有日志文件已被内核重写、
/// 偏移失效时才退回内存里保存的首行，并说明原因。
fn body(block: &LogBlock, content: Option<&str>, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let mut area = v_flex().w_full().gap_2();
    if content.is_none() && block.expandable() {
        area = area.child(
            div()
                .text_color(theme.warning)
                .child("日志文件已被重写，只能展示首行内容。"),
        );
    }

    let text = content.unwrap_or(block.text.as_ref());
    area.child(
        div()
            .id("log-detail-content")
            .w_full()
            .max_h(CONTENT_HEIGHT)
            .overflow_y_scrollbar()
            .font_family(theme.mono_font_family.clone())
            .child(text.to_string()),
    )
    .into_any_element()
}
