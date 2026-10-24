//! sing-box 配置生成与真实运行的集成测试支持。
//!
//! 订阅源放在 `assets/source`, 该目录被 git 忽略且只保留 `.gitkeep`, 因此本地没有
//! 源时测试只输出警告。
//!
//! 生成链路: `lib-pick` 提取节点 -> `lib-converter` 转换配置 -> 结构校验 ->
//! `lib-singbox` 调用内核 `check` 校验; 真实运行链路在此基础上把配置改成随机端口
//! 并启动内核, 走入口代理用 `lib-fetch` 探测外网。

pub mod kernel;
pub mod pipeline;
pub mod probe;
pub mod rules;
pub mod runtime;
pub mod verify;

pub use kernel::Kernel;
pub use pipeline::{Pipeline, Scenario};
pub use runtime::Runtime;
