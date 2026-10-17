pub mod config;
pub mod config_file;
pub mod config_sync;
pub mod file;
pub mod rule;
pub mod rule_file;
pub mod rule_sync;
mod scheduler;
pub mod subscribe;
pub mod subscribe_sync;
pub mod timer;

/// 由 `build.rs` 生成的定时任务名称枚举。
mod timer_name {
    include!(concat!(env!("OUT_DIR"), "/timer_name.rs"));
}
