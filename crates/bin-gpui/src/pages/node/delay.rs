use gpui_kit::component::Theme;
use gpui_kit::*;

/// 正常延迟上限，单位毫秒。超过即视为偏慢。
const NORMAL_MAX: i32 = 300;

/// 节点延迟分档。
///
/// 分档同时决定展示文案与颜色：左侧分组行与右侧节点行共用同一套口径，
/// 同一条延迟在两处不会显示成不同档次。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Delay {
    /// 正常
    Normal(i32),
    /// 偏慢
    Slow(i32),
    /// 尚未测速或测速失败
    Unknown,
}

impl Delay {
    /// 由内核返回的延迟分档，`0` 与负值表示没有可用的测速结果。
    pub fn of(delay: i32) -> Self {
        match delay {
            delay if delay <= 0 => Self::Unknown,
            delay if delay <= NORMAL_MAX => Self::Normal(delay),
            delay => Self::Slow(delay),
        }
    }

    /// 展示文案。
    pub fn text(self) -> SharedString {
        match self {
            Self::Normal(delay) | Self::Slow(delay) => format!("{delay}ms").into(),
            Self::Unknown => "—".into(),
        }
    }

    /// 文案颜色：正常取成功色、偏慢取警告色、无数据取次要文字色。
    pub fn color(self, theme: &Theme) -> Hsla {
        match self {
            Self::Normal(_) => theme.success,
            Self::Slow(_) => theme.warning,
            Self::Unknown => theme.muted_foreground,
        }
    }

    /// 渲染成一行带颜色的延迟文案。
    pub fn render(self, theme: &Theme) -> impl IntoElement {
        div()
            .flex_shrink_0()
            .text_color(self.color(theme))
            .child(self.text())
    }
}
