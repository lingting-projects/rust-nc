//! 连接数据的存储。
//!
//! 订阅线程把内核推送的增量事件累积成一份不可变快照，读取方直接取 [`Arc`]，
//! 因此一次读取不会复制连接数据，也不会与推送互相阻塞。

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::Instant;

use arc_swap::ArcSwapOption;
use indexmap::IndexMap;

use super::record::ConnectionInfo;
use super::snapshot::{ConnectionRecord, ConnectionSnapshot};
use super::stream;
use crate::grpc::{ConnectionEvent, ConnectionEventType, ConnectionEvents, SingBoxGrpc};

/// 已关闭连接的保留上限，超出后丢弃最早关闭的记录。
const CLOSED_LIMIT: usize = 300;

/// 连接数据存储。
///
/// 进程内只有一份，由订阅线程写入、界面读取。
pub struct ConnectionStore {
    /// 最新快照，读取方拿到的都是共享引用。
    snapshot: ArcSwapOption<ConnectionSnapshot>,
    inner: Mutex<Inner>,
    /// 内核客户端注入或撤销时的唤醒信号。
    wake: Condvar,
}

/// 存储内部数据。
#[derive(Default)]
struct Inner {
    /// 内核客户端，内核未运行时为空。
    grpc: Option<Arc<SingBoxGrpc>>,
    /// 订阅线程是否已经启动，线程常驻，内核启停只换客户端。
    started: bool,
    /// 尚未生成快照的连接数据。
    state: State,
}

impl Default for ConnectionStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionStore {
    /// 创建空存储。
    pub fn new() -> Self {
        Self {
            snapshot: ArcSwapOption::empty(),
            inner: Mutex::new(Inner::default()),
            wake: Condvar::new(),
        }
    }

    /// 当前快照，尚未收到推送时为 `None`。
    pub fn snapshot(&self) -> Option<Arc<ConnectionSnapshot>> {
        self.snapshot.load_full()
    }

    /// 注入内核客户端并启动订阅线程，传入 `None` 表示内核已停止。
    ///
    /// 线程只在首次注入时启动，之后常驻等待客户端；内核重启后换掉客户端即可继续接收推送。
    pub(crate) fn subscribe(self: &Arc<Self>, grpc: Option<Arc<SingBoxGrpc>>) {
        let start = {
            let mut inner = self.lock();
            inner.grpc = grpc;
            let start = !inner.started;
            inner.started = true;
            start
        };

        if start {
            stream::spawn(self.clone());
        }
        self.wake.notify_all();
    }

    /// 清空内存中缓存的全部连接。
    ///
    /// 活动连接与已关闭连接一并丢弃，之后只保留新推送的数据。
    /// 订阅不断开：内核下发的更新事件会带着累计流量重新填回对应连接，
    /// 因此清空不影响后续推送的准确性。
    pub fn clear(&self) {
        let mut inner = self.lock();
        inner.state = State::default();
        self.snapshot.store(None);
    }

    /// 丢弃连接数据并把订阅线程与数据一起复位，内核停止或重启时调用。
    ///
    /// 与 [`clear`](Self::clear) 的差别在于快照直接换成空快照而不是 `None`：
    /// 此时内核已经没有连接可读，界面应立刻看到空列表而不是「加载中」。
    pub(crate) fn reset(&self) {
        let mut inner = self.lock();
        inner.state = State::default();
        self.snapshot.store(Some(Arc::default()));
    }

    /// 等待并取出当前内核客户端，客户端被撤销时继续等待。
    pub(crate) fn wait_grpc(&self) -> Arc<SingBoxGrpc> {
        let mut inner = self.lock();
        loop {
            if let Some(grpc) = inner.grpc.clone() {
                return grpc;
            }
            inner = self
                .wake
                .wait(inner)
                .unwrap_or_else(|error| error.into_inner());
        }
    }

    /// 当前客户端是否仍是 `grpc`，订阅断开后据此决定是否重连。
    pub(crate) fn same_grpc(&self, grpc: &Arc<SingBoxGrpc>) -> bool {
        self.lock()
            .grpc
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, grpc))
    }

    /// 应用一批推送事件，并据此生成新快照。
    pub(crate) fn apply(&self, events: ConnectionEvents) {
        let mut inner = self.lock();
        let now = Instant::now();

        // 内核在订阅建立或重建后先推一次全量状态，此时旧数据已经作废。
        if events.reset {
            inner.state = State::default();
        }
        for event in events.events {
            inner.state.event(event);
        }

        let previous = self.snapshot.load_full().unwrap_or_default();
        let snapshot = inner.state.snapshot(now, &previous);
        self.snapshot.store(Some(snapshot));
    }

    /// 按当前数据重新生成快照。
    ///
    /// 内核只在有流量时才推送更新，空闲连接若一直等推送，速率会停在最后一次的瞬时值上；
    /// 订阅线程因此在没有推送时也按间隔刷新一次，让空闲连接的速率回落到 0。
    pub(crate) fn refresh(&self) {
        self.apply(ConnectionEvents::default());
    }

    /// 取内部数据，锁被污染时继续使用其内容。
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|error| error.into_inner())
    }
}

/// 尚未生成快照的连接数据。
///
/// 活动连接按内核推送顺序保存，已关闭的连接单独记录，最近关闭的排在前面。
struct State {
    /// 活动连接，键为连接 id。
    active: IndexMap<String, ConnectionInfo>,
    /// 已关闭的连接，最近关闭的在前。
    closed: VecDeque<ConnectionRecord>,
    /// 上次生成快照的时刻，用于把两次快照之间的累计流量差换算成速率。
    last: Instant,
}

impl Default for State {
    fn default() -> Self {
        Self {
            active: IndexMap::new(),
            closed: VecDeque::new(),
            last: Instant::now(),
        }
    }
}

impl State {
    /// 应用单个事件。
    fn event(&mut self, event: ConnectionEvent) {
        match ConnectionInfo::kind(&event) {
            ConnectionEventType::ConnectionEventClosed => self.close(event),
            _ => self.open(event),
        }
    }

    /// 新增或更新一条连接。
    ///
    /// 内核的更新事件只带流量增量、不带连接数据，此时沿用已有的连接信息。
    fn open(&mut self, event: ConnectionEvent) {
        if let Some(connection) = event.connection {
            let info = ConnectionInfo::of(connection);
            // 订阅建立时的全量状态里, 已关闭的连接也以新增事件下发, 靠关闭时间区分。
            if info.closed_at > 0 {
                self.active.shift_remove(&info.id);
                self.push_closed(ConnectionRecord {
                    info,
                    download: 0.,
                    upload: 0.,
                    closed: true,
                });
                return;
            }

            self.active.insert(info.id.clone(), info);
            return;
        }

        // 只带增量的事件: 累加到连接的累计流量上。
        let Some(info) = self.active.get_mut(&event.id) else {
            return;
        };
        info.download = info
            .download
            .saturating_add(event.downlink_delta.max(0) as u64);
        info.upload = info.upload.saturating_add(event.uplink_delta.max(0) as u64);
    }

    /// 关闭一条连接，保留在列表里供查看。
    fn close(&mut self, event: ConnectionEvent) {
        let tracked = self.active.shift_remove(&event.id);
        let mut info = match (event.connection, tracked) {
            // 内核在关闭事件里也带上连接数据, 优先用推送的这份。
            (Some(connection), _) => ConnectionInfo::of(connection),
            (None, Some(info)) => info,
            (None, None) => return,
        };
        if event.closed_at > 0 {
            info.closed_at = event.closed_at;
        }

        self.push_closed(ConnectionRecord {
            info,
            // 关闭后不再有流量增量，速率固定为 0，避免停在关闭前的瞬时值上。
            download: 0.,
            upload: 0.,
            closed: true,
        });
    }

    /// 追加一条已关闭连接，超出上限时丢弃最早关闭的。
    fn push_closed(&mut self, record: ConnectionRecord) {
        self.closed.retain(|item| item.info.id != record.info.id);
        self.closed.push_front(record);
        self.closed.truncate(CLOSED_LIMIT);
    }

    /// 生成快照。
    ///
    /// 速率取本次与上次快照的累计流量差除以实际间隔，因此与内核推送的间隔无关；
    /// 首次出现的连接没有参照，速率按 0 处理。
    fn snapshot(&mut self, now: Instant, previous: &ConnectionSnapshot) -> Arc<ConnectionSnapshot> {
        let seconds = now.duration_since(self.last).as_secs_f64().max(0.001);
        self.last = now;

        let mut before: HashMap<&str, (u64, u64)> = HashMap::with_capacity(previous.active);
        for row in &previous.rows[..previous.active] {
            before.insert(row.info.id.as_str(), (row.info.download, row.info.upload));
        }

        let active = self.active.len();
        let mut rows = Vec::with_capacity(active + self.closed.len());
        rows.extend(self.active.values().map(|info| {
            let (download, upload) = match before.get(info.id.as_str()) {
                Some((last_download, last_upload)) => (
                    info.download.saturating_sub(*last_download) as f64 / seconds,
                    info.upload.saturating_sub(*last_upload) as f64 / seconds,
                ),
                None => (0., 0.),
            };

            ConnectionRecord {
                info: info.clone(),
                download,
                upload,
                closed: false,
            }
        }));
        rows.extend(self.closed.iter().cloned());

        Arc::new(ConnectionSnapshot { rows, active })
    }
}
