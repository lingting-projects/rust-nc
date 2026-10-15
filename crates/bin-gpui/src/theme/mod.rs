use std::rc::Rc;

use gpui_kit::component::{Theme, ThemeConfig, ThemeConfigColors, ThemeMode};
use gpui_kit::*;

/// 主色（antd `colorPrimary`，旧界面的拂晓蓝）。
const PRIMARY: &str = "#1890ff";
/// 主色悬停（antd `colorPrimaryHover`）。
const PRIMARY_HOVER: &str = "#40a9ff";
/// 主色按下（antd `colorPrimaryActive`）。
const PRIMARY_ACTIVE: &str = "#096dd9";
/// 主色浅底（antd `colorPrimaryBg`）。
const PRIMARY_BG: &str = "#e6f7ff";
/// 主色浅边框（antd `colorPrimaryBorder`）。
const PRIMARY_BORDER: &str = "#91d5ff";
/// 主色的 10% 透明度版本，供会被限制透明度的选中态使用。
const PRIMARY_TINT: &str = "#1890ff1a";
/// 主色阶梯的中间两级（antd `blue-4`、`blue-2`），供图表使用。
const CHART_BLUE_3: &str = "#69c0ff";
const CHART_BLUE_5: &str = "#bae7ff";

/// 成功色（antd `colorSuccess`）。
const SUCCESS: &str = "#52c41a";
/// 成功色悬停（antd `colorSuccessHover`）。
const SUCCESS_HOVER: &str = "#73d13d";
/// 成功色按下（antd `colorSuccessActive`）。
const SUCCESS_ACTIVE: &str = "#389e0d";

/// 警告色（antd `colorWarning`）。
const WARNING: &str = "#faad14";
/// 警告色悬停（antd `colorWarningHover`）。
const WARNING_HOVER: &str = "#ffc53d";
/// 警告色按下（antd `colorWarningActive`）。
const WARNING_ACTIVE: &str = "#d48806";

/// 错误色（antd `colorError`）。
const ERROR: &str = "#ff4d4f";
/// 错误色悬停（antd `colorErrorHover`）。
const ERROR_HOVER: &str = "#ff7875";
/// 错误色按下（antd `colorErrorActive`）。
const ERROR_ACTIVE: &str = "#d9363e";

/// 主要文字色（antd `colorText`）。
const TEXT: &str = "#000000e0";
/// 次级文字色（antd `colorTextSecondary`）。
const TEXT_SECONDARY: &str = "#000000a6";
/// 三级文字色（antd `colorTextTertiary`）。
const TEXT_TERTIARY: &str = "#00000073";

/// 边框色（antd `colorBorder`）。
const BORDER: &str = "#d9d9d9";
/// 分隔线色（antd `colorSplit`）。
const BORDER_SECONDARY: &str = "#f0f0f0";

/// 容器底色（antd `colorBgContainer`）。
const BG_CONTAINER: &str = "#ffffff";
/// 布局底色（antd `colorBgLayout`）。
const BG_LAYOUT: &str = "#f5f5f5";
/// 遮罩色（antd `colorBgMask`）。
const BG_MASK: &str = "#00000073";

/// 最重的填充色（antd `colorFill`）。
const FILL: &str = "#00000026";
/// 次级填充色（antd `colorFillSecondary`）。
const FILL_SECONDARY: &str = "#0000000f";
/// 三级填充色（antd `colorFillTertiary`）。
const FILL_TERTIARY: &str = "#0000000a";
/// 四级填充色（antd `colorFillQuaternary`）。
const FILL_QUATERNARY: &str = "#00000005";
/// 更重的填充色（antd 关闭态 Switch 的底色），用于需要明显可见的灰底。
const FILL_HEAVY: &str = "#00000040";

/// 纯白：主色与状态色按钮上的文字、滑块把手。
const WHITE: &str = "#ffffff";

/// 按 `字段 => 色值` 逐项写入色板。
macro_rules! palette {
    ($colors:expr, $($field:ident => $value:expr),* $(,)?) => {
        $(
            $colors.$field = Some($value.into());
        )*
    };
}

/// 应用主题。必须在 [`gpui_kit::init`] 之后、创建窗口之前调用。
pub fn apply(cx: &mut App) {
    let config = Rc::new(config());
    Theme::update(cx, |theme| theme.apply_config(&config));
}

/// 主题配置：antd 的圆角、阴影与浅色色板。
fn config() -> ThemeConfig {
    ThemeConfig {
        name: "Antd Dawn Blue".into(),
        mode: ThemeMode::Light,
        radius: Some(6),
        radius_lg: Some(8),
        shadow: Some(true),
        colors: colors(),
        ..Default::default()
    }
}

/// 浅色色板。
///
/// `ThemeConfigColors` 的基础色板字段（`base.*`）不对外公开，无法用结构体字面量
/// 整体构造，因此用 `palette!` 逐项写入；字段与色值成对书写，便于与 antd 的
/// token 对照。
fn colors() -> ThemeConfigColors {
    let mut colors = ThemeConfigColors::default();
    palette! { colors,
        // 基础
        background => BG_CONTAINER,
        foreground => TEXT,
        border => BORDER,
        input => BORDER,
        ring => PRIMARY_BORDER,
        caret => TEXT,
        selection => PRIMARY,
        overlay => BG_MASK,
        drag_border => PRIMARY,
        drop_target => PRIMARY_TINT,
        window_border => BORDER,

        // 主色
        primary => PRIMARY,
        primary_foreground => WHITE,
        primary_hover => PRIMARY_HOVER,
        primary_active => PRIMARY_ACTIVE,

        // 中性容器与文字：填充色都比布局底深，选中与悬浮才不会和背景糊在一起
        secondary => FILL_TERTIARY,
        secondary_foreground => TEXT,
        secondary_hover => FILL_SECONDARY,
        secondary_active => FILL,
        accent => FILL_QUATERNARY,
        accent_foreground => TEXT,
        muted => BG_LAYOUT,
        muted_foreground => TEXT_TERTIARY,
        popover => BG_CONTAINER,
        popover_foreground => TEXT,

        // 状态色
        danger => ERROR,
        danger_foreground => WHITE,
        danger_hover => ERROR_HOVER,
        danger_active => ERROR_ACTIVE,
        success => SUCCESS,
        success_foreground => WHITE,
        success_hover => SUCCESS_HOVER,
        success_active => SUCCESS_ACTIVE,
        warning => WARNING,
        warning_foreground => WHITE,
        warning_hover => WARNING_HOVER,
        warning_active => WARNING_ACTIVE,
        info => PRIMARY,
        info_foreground => WHITE,
        info_hover => PRIMARY_HOVER,
        info_active => PRIMARY_ACTIVE,

        // 按钮
        button => BG_CONTAINER,
        button_foreground => TEXT,
        button_hover => FILL_TERTIARY,
        button_active => FILL_SECONDARY,
        button_primary => PRIMARY,
        button_primary_foreground => WHITE,
        button_primary_hover => PRIMARY_HOVER,
        button_primary_active => PRIMARY_ACTIVE,
        button_secondary => BG_CONTAINER,
        button_secondary_foreground => TEXT,
        button_secondary_hover => FILL_TERTIARY,
        button_secondary_active => FILL_SECONDARY,
        button_danger => ERROR,
        button_danger_foreground => WHITE,
        button_danger_hover => ERROR_HOVER,
        button_danger_active => ERROR_ACTIVE,
        button_success => SUCCESS,
        button_success_foreground => WHITE,
        button_success_hover => SUCCESS_HOVER,
        button_success_active => SUCCESS_ACTIVE,
        button_warning => WARNING,
        button_warning_foreground => WHITE,
        button_warning_hover => WARNING_HOVER,
        button_warning_active => WARNING_ACTIVE,
        button_info => PRIMARY,
        button_info_foreground => WHITE,
        button_info_hover => PRIMARY_HOVER,
        button_info_active => PRIMARY_ACTIVE,

        // 链接、分组与折叠面板
        link => PRIMARY,
        link_hover => PRIMARY_HOVER,
        link_active => PRIMARY_ACTIVE,
        accordion => BG_CONTAINER,
        group_box => BG_CONTAINER,
        group_box_foreground => TEXT,
        group_box_title_foreground => TEXT,
        description_list_label => FILL_QUATERNARY,
        description_list_label_foreground => TEXT,

        // 列表与表格
        list => BG_CONTAINER,
        list_hover => FILL_TERTIARY,
        list_active => PRIMARY_TINT,
        list_active_border => PRIMARY,
        list_even => FILL_QUATERNARY,
        list_head => FILL_QUATERNARY,
        table => BG_CONTAINER,
        table_hover => FILL_TERTIARY,
        table_active => PRIMARY_TINT,
        table_active_border => PRIMARY,
        table_even => FILL_QUATERNARY,
        table_head => FILL_QUATERNARY,
        table_head_foreground => TEXT,
        table_foot => FILL_QUATERNARY,
        table_foot_foreground => TEXT,
        table_row_border => BORDER_SECONDARY,

        // 标签页
        tab => BG_CONTAINER,
        tab_foreground => TEXT_SECONDARY,
        tab_active => BG_CONTAINER,
        tab_active_foreground => PRIMARY,
        tab_bar => BG_LAYOUT,
        tab_bar_segmented => BG_LAYOUT,

        // 侧边栏与窗口栏
        sidebar => BG_CONTAINER,
        sidebar_foreground => TEXT,
        sidebar_border => BORDER_SECONDARY,
        sidebar_accent => PRIMARY_BG,
        sidebar_accent_foreground => PRIMARY,
        sidebar_primary => PRIMARY,
        sidebar_primary_foreground => WHITE,
        title_bar => BG_LAYOUT,
        title_bar_border => BORDER_SECONDARY,
        status_bar => BG_LAYOUT,
        status_bar_border => BORDER_SECONDARY,

        // 控件
        progress_bar => PRIMARY,
        skeleton => FILL_SECONDARY,
        slider_bar => PRIMARY_BORDER,
        slider_thumb => WHITE,
        switch => FILL_HEAVY,
        switch_thumb => WHITE,
        scrollbar => "#00000000",
        scrollbar_thumb => FILL,
        scrollbar_thumb_hover => FILL_HEAVY,

        // 图表：主色由浅到深的蓝色阶梯
        chart_1 => PRIMARY,
        chart_2 => PRIMARY_HOVER,
        chart_3 => CHART_BLUE_3,
        chart_4 => PRIMARY_BORDER,
        chart_5 => CHART_BLUE_5,
        chart_bullish => SUCCESS,
        chart_bearish => ERROR,
        chart_grid => BORDER_SECONDARY,
    }
    colors
}
