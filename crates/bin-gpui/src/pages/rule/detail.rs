//! 规则详情弹窗。
//!
//! 规则内容是「类型,值」逐行的文本，整段贴出来难以核对，因此按类型分好类，
//! 并提供关键字过滤；行数可能上万，列表按固定行高虚拟滚动，只渲染可见部分。

use std::rc::Rc;

use gpui_kit::base::{StyledExt as _, h_flex, v_flex};
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants as _;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{ActiveTheme as _, Selectable as _, Sizable as _, WindowExt as _};
use gpui_kit::*;

use lingting_gpui_kit::ex_dialog::ExDialog;
use lingting_gpui_kit::virtual_card::VirtualCard;

/// 详情弹窗宽度，单位为 rem，随界面缩放一起变化。
const DIALOG_WIDTH: Rems = rems(48.);
/// 列表高度，超出后虚拟滚动。
const LIST_HEIGHT: Rems = rems(24.);
/// 列表行高：行高固定，虚拟列表才能按行排布。
const ROW_HEIGHT: Rems = rems(2.);

/// 一行规则在原文里的归属。
enum Kind {
    /// `#` 开头的注释。
    Comment,
    /// `类型,值`，没有逗号时整行既作类型也作值。
    Rule {
        rule_type: SharedString,
        value: SharedString,
    },
}

/// 原文里的一行，空行在解析时已被剔除。
struct Line {
    /// 去掉首尾空白后的原始行。
    raw: SharedString,
    kind: Kind,
}

/// 分类项：各类型、注释与原文。
#[derive(Clone, PartialEq, Eq)]
enum Category {
    RuleType(SharedString),
    Comment,
    Raw,
}

impl Category {
    /// 分类按钮上的文案。
    fn label(&self) -> SharedString {
        match self {
            Self::RuleType(rule_type) => rule_type.clone(),
            Self::Comment => "注释".into(),
            Self::Raw => "原文".into(),
        }
    }
}

/// 打开规则详情弹窗。
///
/// 内容由调用方在点击时拷贝进来，展示的是点击那一刻的快照，
/// 不会因为列表刷新而中途改内容。
pub fn open(name: String, content: String, window: &mut Window, cx: &mut App) {
    let width = DIALOG_WIDTH.to_pixels(window.rem_size());
    let detail = cx.new(|cx| {
        let query = cx.new(|cx| InputState::new(window, cx).placeholder("搜索规则"));
        Detail::new(&content, query, cx)
    });

    window.open_dialog(cx, move |dialog, _, _| {
        ExDialog::from(dialog)
            .title(name.clone())
            .w(width)
            .content({
                let detail = detail.clone();
                move |content_panel, _, _| content_panel.child(detail.clone())
            })
            .footer(
                h_flex().justify_end().child(
                    Button::new("rule-detail-close")
                        .label("关闭")
                        .on_click(|_, window, cx| window.close_dialog(cx)),
                ),
            )
            .into_dialog()
    });
}

/// 详情弹窗的内容：分类、搜索与可见行。
///
/// 作为独立实体存在：切换分类与输入关键字只更新自己，不必重建弹窗。
pub struct Detail {
    /// 全部非空行，按原文顺序。
    lines: Vec<Line>,
    /// 全部可选分类项：各类型、注释（有注释行时）、原文。
    ///
    /// 分类项与关键字无关：关键字只筛当前分类下的行，不改变可选的分类。
    categories: Vec<Category>,
    /// 当前分类。
    category: Category,
    /// 搜索输入框。
    query: Entity<InputState>,
    /// 当前分类与关键字下要展示的行，随二者变化重建。
    rows: Rc<Vec<SharedString>>,
    /// 输入变化订阅，实体存活期间一直保留。
    _query_subscription: Subscription,
}

impl Detail {
    /// 解析原文并建立详情，默认展示原文分类。
    fn new(content: &str, query: Entity<InputState>, cx: &mut Context<Self>) -> Self {
        let lines = parse(content);
        let categories = categories(&lines);
        let _query_subscription = cx.subscribe(&query, |detail, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                detail.refresh(cx);
                cx.notify();
            }
        });

        let mut detail = Self {
            lines,
            categories,
            category: Category::Raw,
            query,
            rows: Rc::new(Vec::new()),
            _query_subscription,
        };
        detail.refresh(cx);
        detail
    }

    /// 切换分类。
    fn select(&mut self, category: Category, cx: &mut Context<Self>) {
        if self.category == category {
            return;
        }
        self.category = category;
        self.refresh(cx);
        cx.notify();
    }

    /// 按当前分类与关键字重建可见行。
    ///
    /// 关键字只筛当前分类下的行，比较区分大小写；过滤在状态变化时完成，
    /// 列表每帧只按结果渲染，避免滚动时重复匹配全部行。
    fn refresh(&mut self, cx: &mut Context<Self>) {
        let keyword = self.query.read(cx).value().trim().to_string();
        let rows: Vec<SharedString> = self
            .lines
            .iter()
            .filter_map(|line| text(&self.category, line))
            .filter(|text| keyword.is_empty() || text.contains(&keyword))
            .collect();
        self.rows = Rc::new(rows);
    }

    /// 分类按钮：当前分类显示选中态，点击即切换。
    fn categories(&self, cx: &mut Context<Self>) -> AnyElement {
        h_flex()
            .w_full()
            .flex_wrap()
            .gap_1()
            .children(self.categories.iter().map(|category| {
                let selected = *category == self.category;
                let id = format!("rule-detail-category:{}", category.label());
                let target = category.clone();

                Button::new(id)
                    .ghost()
                    .small()
                    .label(category.label())
                    .selected(selected)
                    .toggled(selected)
                    .on_click(cx.listener(move |detail, _, _, cx| {
                        detail.select(target.clone(), cx);
                    }))
            }))
            .into_any_element()
    }

    /// 规则行列表：固定行高虚拟滚动，一行一条规则。
    /// 规则行列表：固定行高虚拟滚动，一行一条规则。
    fn list(&self, cx: &App) -> AnyElement {
        // 规则没有内容与过滤后没有命中是两种情形，说明文案分开
        let hint = match self.lines.is_empty() {
            true => "该规则还没有内容",
            false => "没有匹配的规则",
        };

        VirtualCard::new("rule-detail-lines")
            .with_ghost()
            .card_height(LIST_HEIGHT)
            .items(self.rows.clone())
            .item_height(ROW_HEIGHT)
            .item_renderer(Rc::new(|text: &SharedString, _, cx| row(text, cx)))
            .empty(empty(hint, cx))
            .into_any_element()
    }
}

impl Render for Detail {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_2()
            .child(self.categories(cx))
            .child(
                Input::new(&self.query)
                    .cleanable(true)
                    .aria_label("搜索规则"),
            )
            .child(self.list(cx))
    }
}

/// 逐行解析规则内容。
///
/// 空行直接忽略；`#` 开头的是注释；其余按第一个 `,` 拆成类型与值，
/// 没有逗号的行整行既作类型也作值，保证原文一行不丢。
fn parse(content: &str) -> Vec<Line> {
    content
        .lines()
        .filter_map(|raw| {
            let raw = raw.trim();
            if raw.is_empty() {
                return None;
            }

            let kind = match raw.starts_with('#') {
                true => Kind::Comment,
                false => match raw.split_once(',') {
                    Some((rule_type, value)) => Kind::Rule {
                        rule_type: rule_type.trim().into(),
                        value: value.trim().into(),
                    },
                    None => Kind::Rule {
                        rule_type: raw.into(),
                        value: raw.into(),
                    },
                },
            };
            Some(Line {
                raw: raw.into(),
                kind,
            })
        })
        .collect()
}

/// 由解析结果得到分类项：各类型按首次出现顺序，随后是有内容的注释与原文。
///
/// 没有任何行的分类不展示，因此只有注释可能缺席：类型由行本身得出，一定有内容；
/// 原文始终保留，用来判断规则内容是空还是解析异常。
fn categories(lines: &[Line]) -> Vec<Category> {
    let mut types: Vec<SharedString> = Vec::new();
    let mut has_comment = false;
    for line in lines {
        match &line.kind {
            Kind::Comment => has_comment = true,
            Kind::Rule { rule_type, .. } => {
                if !types.contains(rule_type) {
                    types.push(rule_type.clone());
                }
            }
        }
    }

    types
        .into_iter()
        .map(Category::RuleType)
        .chain(has_comment.then_some(Category::Comment))
        .chain([Category::Raw])
        .collect()
}

/// 一行在某分类下要展示的文本，不属于该分类时为 `None`。
///
/// 类型分类展示去掉类型后的值，注释与原文展示整行。
fn text(category: &Category, line: &Line) -> Option<SharedString> {
    match (category, &line.kind) {
        (Category::Raw, _) => Some(line.raw.clone()),
        (Category::Comment, Kind::Comment) => Some(line.raw.clone()),
        (Category::RuleType(wanted), Kind::Rule { rule_type, value }) if wanted == rule_type => {
            Some(value.clone())
        }
        _ => None,
    }
}

/// 列表一行：等宽字体单行展示，过长时省略，行高因此固定。
fn row(text: &SharedString, cx: &App) -> AnyElement {
    div()
        .w_full()
        .px_1()
        .whitespace_nowrap()
        .text_ellipsis()
        .font_family(cx.theme().mono_font_family.clone())
        .child(text.clone())
        .into_any_element()
}

/// 列表为空时的说明，`hint` 区分「规则没有内容」与「过滤后没有命中」。
fn empty(hint: &'static str, cx: &App) -> AnyElement {
    div()
        .text_color(cx.theme().muted_foreground)
        .child(hint)
        .into_any_element()
}
