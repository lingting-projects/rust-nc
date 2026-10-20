mod clash;
mod constants;
pub mod ip;
mod node_contains;
mod platform;
mod rule;
pub mod ruleset;
mod singbox;
mod source;

pub use clash::ClashRule;
pub use node_contains::NodeContains;
pub use platform::{Platform, PlatformConfig};
pub use rule::{Rule, RuleType};
pub use ruleset::RuleSet;
pub use singbox::SingBoxRule;

use anyhow::Result;
use lib_pick::PickNodes;

use crate::source::Source;

/// 将 clash 订阅提取出的节点转换为指定平台的配置。
pub fn clash_to(platform: Platform, nodes: PickNodes, config: PlatformConfig) -> Result<String> {
    build(platform, Source::Clash, nodes, config)
}

/// 将纯净订阅提取出的节点转换为指定平台的配置。
pub fn pure_to(platform: Platform, nodes: PickNodes, config: PlatformConfig) -> Result<String> {
    build(platform, Source::Pure, nodes, config)
}

fn build(
    platform: Platform,
    source: Source,
    nodes: PickNodes,
    config: PlatformConfig,
) -> Result<String> {
    let kernel = platform::Kernel::new(nodes, config);
    match platform {
        Platform::Clash => clash::build(&kernel, source),
        Platform::SingBox => singbox::build(&kernel, source),
    }
}
