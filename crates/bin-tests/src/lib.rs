//! 二进制入口的集成测试支持。
//!
//! 用例位于 `tests/`:
//!
//! - `singbox`: 走 sing-box 内核, 读取 `assets/source` 中的订阅源完成配置生成、
//!   结构校验与真实运行; 支持模块见 [`singbox`]。
//! - `cloudflare`: 走 `bin-cloudflare`, 本地拉起 `wrangler dev` 与一个假订阅服务,
//!   校验分享与订阅转换接口; 支持模块见 [`cloudflare`]。

pub mod cloudflare;
pub mod server;
pub mod singbox;
pub mod source;
pub mod target;

pub use source::Source;
