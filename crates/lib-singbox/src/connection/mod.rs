//! 连接数据。
//!
//! 内核的 `SubscribeConnections` 是长连接：订阅建立后按固定间隔推送新增、更新与
//! 关闭事件。这里用一条常驻订阅把推送累积成内存快照，调用方按需读取共享引用即可，
//! 不必自己建流，也不必自己用累计流量做差分算速率。

mod record;
mod snapshot;
mod store;
mod stream;

pub use record::ConnectionInfo;
pub use snapshot::{ConnectionRecord, ConnectionSnapshot};
pub use store::ConnectionStore;
