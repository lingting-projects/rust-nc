//! `bin-cloudflare` 的集成测试支持。
//!
//! 本地用 `wrangler dev` 拉起 worker 作为测试环境, 校验两类接口:
//!
//! - 分享: `/s/{key}` 按环境变量 `share` 重定向, 参数不匹配或分享键不存在时拒绝;
//! - 转换: `/clash`、`/sing-box` 把订阅地址转换为对应平台的配置, 订阅源由
//!   [`crate::server`] 提供的假订阅服务返回, 不依赖外网。

pub mod convert;
pub mod share;
pub mod worker;

pub use convert::Platform;
pub use worker::Worker;
