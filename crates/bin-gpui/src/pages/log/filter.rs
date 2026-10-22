//! 日志页面的筛选条件。
//!
//! 关键字与等级是两个独立维度：关键字按块全文匹配，等级按块等级取阈值。

use gpui_kit::base::{StyledExt as _, h_flex};
use gpui_kit::component::IndexPath;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants as _;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::searchable_list::SearchableListItem;
use gpui_kit::component::select::{SearchableVec, Select, SelectEvent, SelectState};
use gpui_kit::component::{ActiveTheme as _, IconName};
use gpui_kit::*;
use lingting_gpui_kit::card::Card;

use lib_singbox::{LogLevel, LogSnapshot};

use super::LogPage;
use super::PADDING;

/// 等级下拉框宽度。
const SELECT_WIDTH: Rems = rems(11.);

/// 等级筛选选项：`None` 表示不限等级。
pub const OPTIONS: [(&str, Option<LogLevel>); 7] = [
    ("全部等级", None),
    ("ERROR 及以上", Some(LogLevel::Error)),
    ("WARN 及以上", Some(LogLevel::Warn)),
    ("INFO 及以上", Some(LogLevel::Info)),
    ("DEBUG 及以上", Some(LogLevel::Debug)),
    ("TRACE 及以上", Some(LogLevel::Trace)),
    ("仅续行", Some(LogLevel::Unknown)),
];

/// 等级下拉框的一个选项。
///
/// 选项值就是标签文本，等级在订阅时按标签回查，因此选项自身不必再存一份等级。
#[derive(Clone)]
pub struct LevelOption {
    title: SharedString,
}

impl SearchableListItem for LevelOption {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.title.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.title
    }
}

/// 全部选项，顺序即界面顺序。
pub fn options() -> Vec<LevelOption> {
    OPTIONS
        .iter()
        .map(|(label, _)| LevelOption {
            title: SharedString::from(*label),
        })
        .collect()
}

/// 创建等级下拉框的状态，默认选中「全部等级」。
pub fn create_state(
    window: &mut Window,
    cx: &mut Context<LogPage>,
) -> Entity<SelectState<SearchableVec<LevelOption>>> {
    cx.new(|cx| {
        SelectState::new(
            SearchableVec::new(options()),
            Some(IndexPath::new(0)),
            window,
            cx,
        )
    })
}

/// 把下拉框的选择变化接到页面状态上。
pub fn subscribe(
    state: &Entity<SelectState<SearchableVec<LevelOption>>>,
    cx: &mut Context<LogPage>,
) -> Subscription {
    cx.subscribe(
        state,
        |page, _, event: &SelectEvent<SearchableVec<LevelOption>>, cx| {
            let SelectEvent::Confirm(Some(value)) = event else {
                return;
            };
            let level = OPTIONS
                .iter()
                .find(|(label, _)| *label == value.as_ref())
                .and_then(|(_, level)| *level);
            page.set_level(level, cx);
        },
    )
}

/// 渲染标题栏：搜索框、等级筛选与暂停按钮。
pub fn render(page: &LogPage, cx: &mut Context<LogPage>) -> AnyElement {
    Card::new()
        .flex_shrink_0()
        .content_padding(PADDING)
        .child(
            h_flex()
                .w_full()
                .gap_3()
                .child(
                    div().flex_1().min_w_0().child(
                        Input::new(&page.query())
                            .cleanable(true)
                            .aria_label("搜索日志"),
                    ),
                )
                .child(
                    div().w(SELECT_WIDTH).flex_shrink_0().child(
                        Select::new(&page.level())
                            .id("log-level")
                            .accessibility_label("日志等级"),
                    ),
                )
                .child(pause(page, cx)),
        )
        .into_any_element()
}

/// 暂停/继续按钮。
///
/// 暂停冻结的是视图：读取线程照常推进游标，恢复后直接看到最新内容，
/// 不必补读暂停期间的日志。
fn pause(page: &LogPage, cx: &mut Context<LogPage>) -> AnyElement {
    let paused = page.is_paused();
    let icon = match paused {
        true => IconName::Play,
        false => IconName::Pause,
    };
    let label = match paused {
        true => "继续",
        false => "暂停",
    };

    Button::new("log-pause")
        .ghost()
        .icon(icon)
        .tooltip(label)
        .on_click(cx.listener(|page, _, _, cx| page.toggle_pause(cx)))
        .into_any_element()
}

/// 按关键字与等级筛选快照，返回命中的块下标。
///
/// 只在这里扫一次，列表本身不设过滤条件：虚拟列表每帧都会重新执行过滤，
/// 上万条日志逐帧全量匹配会直接掉帧。
pub fn filter(snapshot: &LogSnapshot, keyword: &str, level: Option<LogLevel>) -> Vec<usize> {
    let keyword = keyword.trim().to_lowercase();
    let indexed = !keyword.is_empty();

    snapshot
        .blocks
        .iter()
        .enumerate()
        .filter(|(_, block)| match level {
            None => true,
            // 续行没有等级前缀，单独筛选才能只看它们
            Some(LogLevel::Unknown) => !block.level.is_known(),
            // 阈值式筛选：等级越大越详细，「WARN 及以上」即不比 WARN 更详细
            Some(min) => block.level.is_known() && block.level <= min,
        })
        .filter(|(_, block)| match indexed {
            true => block.text.to_lowercase().contains(&keyword),
            false => true,
        })
        .map(|(index, _)| index)
        .collect()
}
