//! 实时监控区域。
//!
//! 基础监控占一整行，流量与内存并排各占一半；未选中的分区不渲染。
//! 图表纵向吃掉剩余高度，窗口变高时折线图跟着变高。

use gpui_kit::base::{StyledExt as _, h_flex, v_flex};
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::chart::{AreaChart, LineChart};
use gpui_kit::component::plot::AxisLabelPlacement;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use lingting_gpui_kit::card::{Card, ItemCard};

use super::MonitorSection;
use super::format;
use super::monitor::{Monitor, Point};

/// 渲染整块实时监控区域。
pub fn render(
    monitor: &Monitor,
    sections: &[MonitorSection],
    theme: &gpui_kit::component::Theme,
) -> AnyElement {
    let points = monitor.points();

    let mut rows: Vec<AnyElement> = Vec::new();
    if sections.contains(&MonitorSection::Basic) {
        rows.push(basic(monitor, theme).into_any_element());
    }

    let mut columns: Vec<AnyElement> = Vec::new();
    if sections.contains(&MonitorSection::Traffic) {
        columns.push(traffic(&points, theme).into_any_element());
    }
    if sections.contains(&MonitorSection::Memory) {
        columns.push(memory(&points, theme).into_any_element());
    }
    if !columns.is_empty() {
        rows.push(
            h_flex()
                .w_full()
                .flex_1()
                .min_h_0()
                .items_stretch()
                .gap_4()
                .children(columns)
                .into_any_element(),
        );
    }

    v_flex()
        .size_full()
        .min_h_0()
        .gap_4()
        .children(rows)
        .into_any_element()
}

/// 基础监控：一整行四个数值卡片。
fn basic(monitor: &Monitor, theme: &gpui_kit::component::Theme) -> impl IntoElement {
    let items = [
        ("上传", format::speed(monitor.upload())),
        ("下载", format::speed(monitor.download())),
        ("总上传", format::data_size(monitor.total_upload() as u64)),
        ("总下载", format::data_size(monitor.total_download() as u64)),
    ];

    h_flex()
        .w_full()
        .flex_shrink_0()
        .items_stretch()
        .gap_4()
        .children(items.iter().map(|(label, value)| {
            div().flex_1().min_w_0().h_16().child(
                ItemCard::new(*label).with_center_split(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(*label),
                    div()
                        .text_base()
                        .font_weight(FontWeight::SEMIBOLD)
                        .truncate()
                        .child(value.clone()),
                ),
            )
        }))
}

/// 流量监控：上传与下载两条折线。
///
/// `LineChart` 只画一条线，两条序列用 `AreaChart` 表达，填充压得很淡，
/// 读起来仍是一条折线。
fn traffic(points: &[Point], theme: &gpui_kit::component::Theme) -> impl IntoElement {
    let data = points.to_vec();
    let upload = theme.chart_1;
    let download = theme.chart_2;

    chart_card(
        "流量",
        vec![("上传", upload), ("下载", download)],
        theme,
        AreaChart::new(data)
            .id("console-traffic-chart")
            .x(seconds_ago)
            .y(|point: &Point| point.upload)
            .name("上传")
            .stroke(upload)
            .fill(upload.opacity(0.15))
            .y(|point: &Point| point.download)
            .name("下载")
            .stroke(download)
            .fill(download.opacity(0.15))
            .linear()
            .y_axis(true)
            .y_axis_label_placement(AxisLabelPlacement::Inside)
            .y_tick_count(4)
            .x_tick_count(4)
            .y_tick_format(|value| format::data_size(value.max(0.) as u64))
            .tooltip_title(ago_title)
            .tooltip_value(|_, index, value| {
                let name = if index == 0 { "上传" } else { "下载" };
                format!("{name} {}", format::speed(value)).into()
            })
            .into_any_element(),
    )
}

/// 内存监控：内存占用一条折线。
fn memory(points: &[Point], theme: &gpui_kit::component::Theme) -> impl IntoElement {
    let stroke = theme.chart_1;

    chart_card(
        "内存",
        vec![("内存", stroke)],
        theme,
        LineChart::new(points.to_vec())
            .id("console-memory-chart")
            .x(seconds_ago)
            .y(|point: &Point| point.memory)
            .name("内存")
            .stroke(stroke)
            .linear()
            .y_axis(true)
            .y_axis_label_placement(AxisLabelPlacement::Inside)
            .y_tick_count(4)
            .x_tick_count(4)
            .y_tick_format(|value| format::data_size(value.max(0.) as u64))
            .tooltip_title(ago_title)
            .tooltip_value(|_, value| format::data_size(value.max(0.) as u64).into())
            .into_any_element(),
    )
}

/// 横轴标签：距离现在多少秒。
fn seconds_ago(point: &Point) -> String {
    point.ago.to_string()
}

/// 悬浮提示的标题。
fn ago_title(point: &Point) -> SharedString {
    if point.ago == 0 {
        "现在".into()
    } else {
        format!("{} 秒前", point.ago).into()
    }
}

/// 图表卡片：标题、图例与折线图。
///
/// 卡片吃掉所在列的剩余高度，两个图表并排时各占一半宽度。
fn chart_card(
    title: &'static str,
    legend: Vec<(&'static str, Hsla)>,
    theme: &gpui_kit::component::Theme,
    chart: AnyElement,
) -> impl IntoElement {
    Card::new()
        .flex_1()
        .min_w_0()
        .min_h_0()
        .content_padding(rems(1.))
        .child(
            v_flex()
                .size_full()
                .min_h_0()
                .gap_3()
                .child(
                    h_flex()
                        .w_full()
                        .flex_shrink_0()
                        .justify_between()
                        .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(title))
                        .child(
                            h_flex()
                                .gap_3()
                                .children(legend.iter().map(|(label, color)| {
                                    h_flex()
                                        .gap_1()
                                        .child(div().size_2().rounded_full().bg(*color))
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(theme.muted_foreground)
                                                .child(*label),
                                        )
                                })),
                        ),
                )
                .child(div().w_full().flex_1().min_h_0().child(chart)),
        )
}
