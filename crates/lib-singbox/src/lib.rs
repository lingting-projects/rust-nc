//! 内核进程管理。
//!
//! 进程的创建、结束与退出检测全部由一条常驻的监控线程完成，界面线程只读
//! [`Phase`] 与失败原因，两者都存在 `ArcSwap` 里，读取不加锁。
//!
//! 这样拆的原因：创建 36MB 的子进程在首次启动时可能耗时一秒以上，结束进程同样要等
//! 系统回收；这些慢 IO 若发生在持锁区间内，界面每帧调用的状态查询就会被一起阻塞。
//!
//! 实例本体与监控线程的实现见 [`kernel`] 模块，连接数据、内核日志与测速队列各自独立成模块。

mod connection;
#[cfg(feature = "grpc")]
mod grpc;
mod kernel;
mod logs;
mod url_test;

pub use connection::{ConnectionInfo, ConnectionRecord, ConnectionSnapshot, ConnectionStore};
#[cfg(feature = "grpc")]
pub use grpc::{Group, GroupItem, ProcessInfo, SingBoxGrpc};
pub use kernel::{Phase, SingBox, bin_path, create};
pub use logs::{Level as LogLevel, LogBlock, LogSnapshot, LogStore};
pub use url_test::UrlTestQueue;

/// 随包分发的 sing-box 版本。
pub const VERSION: &str = "v1.14.3";

/// 内核进程可执行文件名，位于安装目录下。
pub const BIN_NAME: &str = "lingting-nc-singbox";
