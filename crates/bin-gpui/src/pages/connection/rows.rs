use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::Instant;

use crate::services::ConnectionInfo;

/// 已关闭连接的保留上限，避免长时间运行后列表无限增长。
const CLOSED_LIMIT: usize = 500;

/// 一行连接：连接快照加上本次轮询算出的速率。
#[derive(Clone)]
pub struct ConnectionRow {
    /// 连接本身的数据。
    pub info: ConnectionInfo,
    /// 下载速率，单位 Bytes/s。
    pub download: f64,
    /// 上传速率，单位 Bytes/s。
    pub upload: f64,
    /// 是否已关闭。内核不再推送该连接后由本页记录，速率不再更新。
    pub closed: bool,
}

impl ConnectionRow {
    /// 链路文本，从第一个出站到最后一个出站。
    pub fn chain(&self) -> String {
        match self.info.chain.is_empty() {
            true => "直连".to_string(),
            false => self.info.chain.join(" → "),
        }
    }

    /// 进程名，只取路径的最后一段；取不到进程信息时返回 `-`。
    pub fn process(&self) -> String {
        if self.info.process.is_empty() {
            return "-".to_string();
        }
        let name = self
            .info
            .process
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or_default();
        match name.is_empty() {
            true => self.info.process.clone(),
            false => name.to_string(),
        }
    }

    /// 是否命中关键字：按入口、进程、目标、规则、链路与源地址做包含匹配。
    pub fn matched(&self, keyword: &str) -> bool {
        let keyword = keyword.trim().to_lowercase();
        if keyword.is_empty() {
            return true;
        }

        let fields = [
            self.info.inbound.as_str(),
            self.info.destination.as_str(),
            self.info.rule.as_str(),
            self.info.source.as_str(),
            self.info.process.as_str(),
        ];
        if fields
            .iter()
            .any(|field| field.to_lowercase().contains(&keyword))
        {
            return true;
        }

        self.info
            .chain
            .iter()
            .any(|item| item.to_lowercase().contains(&keyword))
    }
}

/// 上一次轮询时每条连接的累计流量与时刻，用于换算速率。
struct Sample {
    download: u64,
    upload: u64,
    at: Instant,
}

/// 由连续两次快照差分出速率的计算器。
///
/// 内核只提供累计流量，速率必须靠两次采样的差除以实际间隔得到；
/// 首次出现的连接没有参照，速率按 0 处理，下一轮轮询即可得到真实值。
#[derive(Default)]
struct SpeedMeter {
    samples: HashMap<String, Sample>,
}

impl SpeedMeter {
    /// 用一次快照算出每条连接当前的速率。
    fn measure(&mut self, connections: Vec<ConnectionInfo>) -> Vec<ConnectionRow> {
        let now = Instant::now();
        let mut samples = HashMap::with_capacity(connections.len());

        let rows = connections
            .into_iter()
            .map(|info| {
                let (download, upload) = match self.samples.get(&info.id) {
                    Some(last) => {
                        let seconds = now.duration_since(last.at).as_secs_f64().max(0.001);
                        (
                            info.download.saturating_sub(last.download) as f64 / seconds,
                            info.upload.saturating_sub(last.upload) as f64 / seconds,
                        )
                    }
                    None => (0., 0.),
                };

                samples.insert(
                    info.id.clone(),
                    Sample {
                        download: info.download,
                        upload: info.upload,
                        at: now,
                    },
                );

                ConnectionRow {
                    info,
                    download,
                    upload,
                    closed: false,
                }
            })
            .collect();

        self.samples = samples;
        rows
    }

    /// 清空采样，暂停或重新读取前调用，避免拿旧数据算速率。
    fn reset(&mut self) {
        self.samples.clear();
    }
}

/// 连接列表：活动连接在前、已关闭的连接追加在后。
///
/// 内核只推送仍处于打开状态的连接，无法查询已关闭的连接，因此「关闭」由本页自己记录：
/// 上一轮还在、这一轮消失的连接即视为已关闭，保留在列表里供查看。
/// 该记录只覆盖本页打开期间关闭的连接，重启页面后不再出现。
pub struct ConnectionList {
    /// 活动连接，已带速率。
    active: Vec<ConnectionRow>,
    /// 已关闭的连接，最近关闭的在前。
    closed: Vec<ConnectionRow>,
    /// 供表格使用的合并列表：活动连接在前、已关闭的连接在后。
    rows: Rc<Vec<ConnectionRow>>,
    meter: SpeedMeter,
}

impl Default for ConnectionList {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionList {
    /// 创建空列表。
    pub fn new() -> Self {
        Self {
            active: Vec::new(),
            closed: Vec::new(),
            rows: Rc::new(Vec::new()),
            meter: SpeedMeter::default(),
        }
    }

    /// 用一次连接快照更新列表。
    pub fn update(&mut self, connections: Vec<ConnectionInfo>) {
        let active = self.meter.measure(connections);

        // 上一轮存在、这一轮消失的连接视为已关闭；连接 id 唯一，不必去重。
        let current: HashSet<&str> = active.iter().map(|row| row.info.id.as_str()).collect();
        let closed: Vec<ConnectionRow> = self
            .active
            .iter()
            .filter(|row| !current.contains(row.info.id.as_str()))
            .map(|row| ConnectionRow {
                // 关闭后不再有流量增量，速率固定为 0，避免停在关闭前的瞬时值上。
                download: 0.,
                upload: 0.,
                closed: true,
                ..row.clone()
            })
            .collect();

        self.closed.splice(0..0, closed);
        self.closed.truncate(CLOSED_LIMIT);
        self.active = active;
        self.rebuild();
    }

    /// 清空全部数据，活动与已关闭的连接都不再展示。
    pub fn clear(&mut self) {
        self.meter.reset();
        self.active.clear();
        self.closed.clear();
        self.rebuild();
    }

    /// 重新读取前丢掉速率采样，避免用跨越暂停或断开操作的时间间隔算速率。
    pub fn reset_speed(&mut self) {
        self.meter.reset();
    }

    /// 供表格使用的合并列表。
    pub fn rows(&self) -> Rc<Vec<ConnectionRow>> {
        self.rows.clone()
    }

    /// 合并列表中活动连接的数量，之后的部分是已关闭的连接。
    pub fn active_count(&self) -> usize {
        self.active.len()
    }

    /// 是否没有任何数据。
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// 按 id 查找一行，活动与已关闭的都包含在内。
    pub fn find(&self, id: &str) -> Option<&ConnectionRow> {
        self.rows.iter().find(|row| row.info.id == id)
    }

    /// 重建合并列表。
    fn rebuild(&mut self) {
        let mut rows = Vec::with_capacity(self.active.len() + self.closed.len());
        rows.extend(self.active.iter().cloned());
        rows.extend(self.closed.iter().cloned());
        self.rows = Rc::new(rows);
    }
}

/// 关键字命中的行下标。
pub fn matched(rows: &[ConnectionRow], keyword: &str) -> Vec<usize> {
    (0..rows.len())
        .filter(|ix| rows[*ix].matched(keyword))
        .collect()
}
