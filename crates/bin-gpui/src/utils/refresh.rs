use std::collections::HashSet;
use std::rc::Rc;

use gpui_kit::component::Sizable as _;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::tag::Tag;
use gpui_kit::*;

/// 单条数据的刷新状态。
///
/// 「刷新中」来自同步调度器的在跑任务，另外两种由刷新时间推出：
/// 从未刷新过的时间为 0，其余都算已刷新。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RefreshState {
    /// 未刷新
    Never,
    /// 刷新中
    Refreshing,
    /// 已刷新
    Done,
}

impl RefreshState {
    /// 由刷新时间与是否在刷新中判定状态。
    pub fn of(refresh_time: i64, refreshing: bool) -> Self {
        if refreshing {
            Self::Refreshing
        } else if refresh_time <= 0 {
            Self::Never
        } else {
            Self::Done
        }
    }

    /// 状态文案，颜色之外还要靠文字表达状态。
    pub fn label(self) -> &'static str {
        match self {
            Self::Never => "未刷新",
            Self::Refreshing => "刷新中",
            Self::Done => "已刷新",
        }
    }

    /// 状态标签：刷新中带转圈图标，让等待有可见的进展。
    pub fn render(self) -> AnyElement {
        let tag = match self {
            Self::Never => Tag::secondary(),
            Self::Refreshing => Tag::warning(),
            Self::Done => Tag::success(),
        }
        .small();

        match self {
            Self::Refreshing => tag
                .child(Spinner::new().xsmall())
                .child(self.label())
                .into_any_element(),
            _ => tag.child(self.label()).into_any_element(),
        }
    }
}

/// 「刷新中」集合。
///
/// 同步任务跑在业务层的调度器上，页面只能问「现在有哪些 id 在刷新」；
/// 数据更新事件（含同步任务结束）到达时重问一次，因此不需要定时轮询。
pub struct Refresher {
    pending: Rc<HashSet<i64>>,
}

impl Refresher {
    /// 创建空的刷新状态。
    pub fn new() -> Self {
        Self {
            pending: Rc::new(HashSet::new()),
        }
    }

    /// 刷新中的 id 集合，交给表格渲染状态列。
    pub fn pending(&self) -> Rc<HashSet<i64>> {
        self.pending.clone()
    }

    /// 用调度器的最新结果替换刷新中的 id，返回是否发生变化。
    pub fn update(&mut self, ids: Vec<i64>) -> bool {
        let next: HashSet<i64> = ids.into_iter().collect();
        if next == *self.pending {
            return false;
        }
        self.pending = Rc::new(next);
        true
    }
}

impl Default for Refresher {
    fn default() -> Self {
        Self::new()
    }
}
