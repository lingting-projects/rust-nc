//! sing-box 配置生成的集成测试支持。
//!
//! 用例位于 `tests/`, 订阅源放在 `assets/source`, 该目录被 git 忽略且只保留
//! `.gitkeep`, 因此本地没有源时测试只输出警告。
//!
//! 生成链路: `lib-pick` 提取节点 -> `lib-converter` 转换配置 -> 结构校验 ->
//! `lib-singbox` 调用内核 `check` 校验。

pub mod kernel;
pub mod pipeline;
pub mod rules;
pub mod source;
pub mod verify;

pub use kernel::Kernel;
pub use pipeline::{Pipeline, Scenario};
pub use source::Source;
