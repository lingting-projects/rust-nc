//! 连接快照。
//!
//! 快照整体不可变，由订阅线程用新的一份替换旧的，因此读取方拿到的
//! [`Arc`] 在其存活期间内容固定，不需要加锁，也不需要复制连接数据。

use std::sync::Arc;

use super::record::ConnectionInfo;

/// 列表中的一行：连接数据加上由推送增量算出的速率。
#[derive(Debug, Clone)]
pub struct ConnectionRecord {
    /// 连接本身的数据。
    pub info: ConnectionInfo,
    /// 下载速率，单位 Bytes/s，已关闭的连接为 0。
    pub download: f64,
    /// 上传速率，单位 Bytes/s，已关闭的连接为 0。
    pub upload: f64,
    /// 是否已关闭。
    pub closed: bool,
}

impl ConnectionRecord {
    /// 供界面展示的链路文本，从第一个出站到最后一个出站。
    pub fn chain(&self) -> String {
        match self.info.chain.is_empty() {
            true => "直连".to_string(),
            false => self.info.chain.join(" → "),
        }
    }
}

/// 某一时刻的连接快照。
///
/// 活动连接在前、已关闭的连接在后，已关闭的连接最近关闭的排在前面。
/// 该顺序让状态筛选退化为取其中一段，界面不必再额外排序。
#[derive(Debug, Default)]
pub struct ConnectionSnapshot {
    /// 活动连接在前、已关闭连接在后的合并列表。
    pub rows: Vec<ConnectionRecord>,
    /// 合并列表中活动连接的数量，之后的部分是已关闭的连接。
    pub active: usize,
}

impl ConnectionSnapshot {
    /// 是否没有任何数据。
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// 按 id 查找一行，活动与已关闭的都包含在内。
    pub fn find(&self, id: &str) -> Option<&ConnectionRecord> {
        self.rows.iter().find(|row| row.info.id == id)
    }

    /// 包装为共享引用，供界面直接持有。
    pub fn share(self) -> Arc<Self> {
        Arc::new(self)
    }
}
