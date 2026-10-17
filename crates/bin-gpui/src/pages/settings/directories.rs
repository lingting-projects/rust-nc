use std::path::{Path, PathBuf};
use std::rc::Rc;

use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants as _;
use gpui_kit::component::notification::Notification;
use gpui_kit::component::{ActiveTheme as _, IconName, Sizable as _, WindowExt as _};
use gpui_kit::*;

use lingting_gpui_kit::card::Card;
/// 卡片内边距，与其它页面保持一致。
const PADDING: Rems = rems(1.);

/// 标签列宽度：固定后各行路径从同一条起始线开始。
const LABEL_WIDTH: Rems = rems(5.);

/// 标签与路径之间的间隔。
const COLUMN_GAP: Rems = rems(1.);

/// 各行之间的间隔。
const ROW_GAP: Rems = rems(0.75);

/// 分区标题。
const TITLE: &str = "数据路径";

/// 一个目录：名称与完整路径。
struct DirectoryEntry {
    label: &'static str,
    path: PathBuf,
}

/// 数据路径分区。
///
/// 只读分区：目录在构造时读取一次，界面只做展示，不提供任何可编辑控件；
/// 路径按完整形式展示，窗口变窄时换行而不是截断。
#[derive(Clone, IntoElement)]
pub struct Directories {
    /// 目录列表，读取失败时保留失败原因。
    entries: Rc<Result<Vec<DirectoryEntry>, SharedString>>,
}

impl Directories {
    /// 读取 `lib_core::directorys` 暴露的全部目录。
    pub fn new() -> Self {
        Self {
            entries: Rc::new(read()),
        }
    }
}

impl Default for Directories {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for Directories {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        Card::new()
            .w_full()
            .flex_shrink_0()
            .title_padding(PADDING)
            .content_padding(PADDING)
            .title(div().text_sm().font_weight(FontWeight::MEDIUM).child(TITLE))
            .child(content(self.entries.as_ref(), cx))
    }
}

/// 分区内容：正常时逐行列出目录，读取失败时给出原因。
fn content(entries: &Result<Vec<DirectoryEntry>, SharedString>, cx: &App) -> AnyElement {
    match entries {
        Ok(entries) => v_flex()
            .w_full()
            .gap(ROW_GAP)
            .children(entries.iter().map(|entry| row(entry, cx)))
            .into_any_element(),
        Err(reason) => div()
            .w_full()
            .text_sm()
            .text_color(cx.theme().danger)
            .child(reason.clone())
            .into_any_element(),
    }
}

/// 一行目录：左侧名称，右侧完整路径，行尾一个打开按钮。
fn row(entry: &DirectoryEntry, cx: &App) -> AnyElement {
    let path = entry.path.clone();

    h_flex()
        .w_full()
        .items_start()
        .gap(COLUMN_GAP)
        .child(
            div()
                .w(LABEL_WIDTH)
                .flex_shrink_0()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(entry.label),
        )
        .child(
            // 路径是标识符，用等宽字体与正文区分；`flex_1` 配 `min_w_0` 才能在窄窗口里换行。
            div()
                .flex_1()
                .min_w_0()
                .text_sm()
                .font_family(cx.theme().mono_font_family.clone())
                .child(entry.path.display().to_string()),
        )
        .child(
            Button::new(format!("settings-directory-open:{}", entry.label))
                .ghost()
                .small()
                .icon(IconName::FolderOpen)
                .tooltip("在资源管理器中打开")
                .accessibility_label("在资源管理器中打开")
                .on_click(move |_, window, cx| open(&path, window, cx)),
        )
        .into_any_element()
}

/// 用资源管理器打开目录，失败时提示原因。
fn open(path: &Path, window: &mut Window, cx: &mut App) {
    if let Err(error) = lib_system::shell::open_path(path) {
        window.push_notification(Notification::error(format!("{error:#}")), cx);
    }
}

/// 读取应用目录。
fn read() -> Result<Vec<DirectoryEntry>, SharedString> {
    let directory = lib_core::directorys()
        .map_err(|error| SharedString::from(format!("读取应用目录失败: {error:#}")))?;

    Ok(vec![
        DirectoryEntry {
            label: "数据",
            path: directory.data.clone(),
        },
        DirectoryEntry {
            label: "缓存",
            path: directory.cache.clone(),
        },
        DirectoryEntry {
            label: "临时",
            path: directory.tmp.clone(),
        },
        DirectoryEntry {
            label: "日志",
            path: directory.logs.clone(),
        },
        DirectoryEntry {
            label: "启动",
            path: directory.startup.clone(),
        },
        DirectoryEntry {
            label: "安装",
            path: directory.install.clone(),
        },
        DirectoryEntry {
            label: "配置",
            path: directory.config.clone(),
        },
        DirectoryEntry {
            label: "规则",
            path: directory.rule.clone(),
        },
    ])
}
