//! 内核日志。
//!
//! 内核把日志写进工作目录下的 `kernel.log`，且每次启动都会重写该文件。
//! 这里用一条常驻线程增量读取文件，把解析出的日志块累积成不可变快照，
//! 界面按需取共享引用即可，不必自己维护读游标，也不必复制日志内容。

mod block;
mod store;
mod stream;

pub use block::{Level, LogBlock, LogSnapshot};
pub use store::LogStore;
