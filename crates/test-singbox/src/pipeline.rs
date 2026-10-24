use anyhow::{Context, Result};
use lib_converter::{Platform, PlatformConfig, clash_to, pure_to};
use lib_pick::{KEY_TYPE, PickNodes, PickRemote, pick};

use crate::kernel::Kernel;
use crate::runtime::Runtime;
use crate::source::Source;
use crate::verify;

/// 单个源的生成链路: 提取 -> 转换 -> 结构校验 -> 内核校验。
pub struct Pipeline<'a> {
    source: &'a Source,
    /// 提取结果
    remote: PickRemote,
    /// 每个生成场景产出的配置
    configs: Vec<Config>,
}

/// 一个生成场景的产物。
pub struct Config {
    /// 场景
    pub scenario: Scenario,
    /// 配置文本
    pub body: String,
}

/// 生成场景, 覆盖转换器中的全部分支: tun、fake ip、ipv6 与 geo cn。
#[derive(Debug, Clone, Copy)]
pub struct Scenario {
    /// 场景名称
    pub name: &'static str,
    /// 是否启用 tun
    pub tun: bool,
    /// 是否启用 fake ip
    pub fake_ip: bool,
    /// 是否启用 ipv6
    pub ipv6: bool,
    /// geo cn 直连
    pub geo_cn: bool,
}

/// 全部生成场景, 组合覆盖每个分支的开启与关闭。
pub const SCENARIOS: [Scenario; 4] = [
    Scenario {
        name: "tun-fake-ipv6-geo",
        tun: true,
        fake_ip: true,
        ipv6: true,
        geo_cn: true,
    },
    Scenario {
        name: "tun-route-ipv4",
        tun: true,
        fake_ip: false,
        ipv6: false,
        geo_cn: false,
    },
    Scenario {
        name: "mixed-fake-ipv6-geo",
        tun: false,
        fake_ip: true,
        ipv6: true,
        geo_cn: true,
    },
    Scenario {
        name: "mixed-route-ipv4",
        tun: false,
        fake_ip: false,
        ipv6: false,
        geo_cn: false,
    },
];

impl<'a> Pipeline<'a> {
    /// 按顺序执行链路, 任一步失败即结束, 错误信息带源文件名。
    pub fn run(source: &'a Source, kernel: &Kernel) -> Result<Self> {
        let remote = pick(&source.content, None)?;
        verify::nodes(&remote.nodes)?;

        let mut configs = Vec::new();
        for scenario in SCENARIOS {
            let body = convert(&remote.nodes, kernel.config(scenario)?)?;
            verify::config(&body)?;
            verify::scenario(&body, &scenario)?;
            kernel.check(scenario.name, &body)?;
            configs.push(Config { scenario, body });
        }

        log::info!("[{}] 生成链路校验通过", source.name);
        Ok(Self {
            source,
            remote,
            configs,
        })
    }

    /// 源文件名。
    pub fn name(&self) -> &str {
        &self.source.name
    }

    /// 提取结果。
    pub fn remote(&self) -> &PickRemote {
        &self.remote
    }

    /// 各场景生成的配置。
    pub fn configs(&self) -> &[Config] {
        &self.configs
    }

    /// 取出指定场景的配置, 改成可真实运行的形态。
    pub fn runtime(&self, scenario: &str) -> Result<Runtime> {
        let config = self
            .configs
            .iter()
            .find(|config| config.scenario.name == scenario)
            .with_context(|| format!("未生成场景配置: {scenario}"))?;
        Runtime::new(config.scenario, &config.body)
    }
}

/// 把节点转换为 sing-box 配置, 来源按协议所在字段名判定。
fn convert(nodes: &PickNodes, config: PlatformConfig) -> Result<String> {
    match is_clash(nodes) {
        true => clash_to(Platform::SingBox, nodes.clone(), config),
        false => pure_to(Platform::SingBox, nodes.clone(), config),
    }
}

/// 是否来自 clash 配置, 依据协议所在的字段名。
fn is_clash(nodes: &PickNodes) -> bool {
    nodes
        .nodes
        .first()
        .is_some_and(|node| node.protocol_key == KEY_TYPE)
}
