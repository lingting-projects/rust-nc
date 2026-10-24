use std::collections::HashSet;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui_kit::component::Sizable as _;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::tag::Tag;
use gpui_kit::*;

/// 轮询间隔：刷新中的状态按此间隔重查。
pub const TICK: Duration = Duration::from_secs(1);

/// 一轮轮询的最短持续时间。
///
/// 点同步后到后台任务真正开始之间有空档，太早停下会让「刷新中」一闪而过；
/// 每次查到仍有数据在刷新都会重新计时，因此实际时长只会更长。
const HOLD: Duration = Duration::from_secs(10);

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

/// 「刷新中」集合与轮询节奏。
///
/// 同步任务跑在业务层的调度器上，页面只能问「现在有哪些 id 在刷新」；
/// 为了让「刷新中」有机会自己变成「已刷新」，这里记录刷新中的集合与
/// 本轮轮询的截止时刻：查到仍在刷新的数据就把截止时刻往后推。
pub struct Refresher {
    pending: Rc<HashSet<i64>>,
    /// 轮询截止时刻，`None` 表示不再轮询
    until: Option<Instant>,
    /// 是否已有轮询任务在跑
    polling: bool,
}

impl Refresher {
    /// 创建空的刷新状态。
    pub fn new() -> Self {
        Self {
            pending: Rc::new(HashSet::new()),
            until: None,
            polling: false,
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

    /// 开始一轮轮询：延长截止时刻，返回是否需要派生轮询任务。
    ///
    /// 已在轮询时只延长截止时刻，避免同一时刻跑两个轮询任务。
    pub fn begin(&mut self) -> bool {
        self.until = Some(Instant::now() + HOLD);
        if self.polling {
            return false;
        }
        self.polling = true;
        true
    }
    /// 轮询一拍：刷新集合，仍有数据在刷新时重新计时，返回集合是否变化。
    pub fn tick(&mut self, ids: Vec<i64>) -> bool {
        let changed = self.update(ids);
        if !self.pending.is_empty() {
            self.until = Some(Instant::now() + HOLD);
        }
        changed
    }

    /// 轮询是否继续：还有数据在刷新，或未到截止时刻。
    pub fn alive(&self) -> bool {
        !self.pending.is_empty() || self.until.is_some_and(|until| Instant::now() < until)
    }

    /// 结束轮询，下一次 [`begin`](Self::begin) 会重新派生任务。
    pub fn stop(&mut self) {
        self.polling = false;
        self.until = None;
    }
}

impl Default for Refresher {
    fn default() -> Self {
        Self::new()
    }
}
