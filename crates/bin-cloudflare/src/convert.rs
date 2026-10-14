//! 订阅转换。

use anyhow::{Result, anyhow};
use lib_converter::{Platform, PlatformConfig, Rule, RuleType, clash_to, pure_to};
use lib_core::MultiStringValue;
use lib_fetch::{FetchClient, FetchRequest};
use lib_pick::{KEY_TYPE, PickNodes, PickRemote, pick};
use worker::console_debug;

use crate::params::ConvertParams;

/// 规则文件所在的 gist 前缀。
const GIST_PREFIX: &str =
    "https://gist.githubusercontent.com/lingting/93a4a9ff5d1134aab8ca286bec969436/raw/";

/// 转换结果。
pub struct Converted {
    /// 转换后的配置内容
    pub body: String,
    /// 响应文件名, 取订阅地址的域名
    pub filename: String,
    /// 订阅流量与到期信息
    pub info: Option<String>,
}

/// 远程规则。
fn rule(rule_type: RuleType, name: &str) -> Rule {
    Rule::from_remote(rule_type, format!("{GIST_PREFIX}{name}"))
}

/// sing-box 使用的规则集合。
fn sing_box_rules() -> (Vec<Rule>, Vec<Rule>, Vec<Rule>) {
    (
        vec![
            rule(RuleType::Process, "sing.direct.p"),
            rule(RuleType::Other, "sing.direct.np"),
            rule(RuleType::Ip, "sing.direct.ip"),
        ],
        vec![
            rule(RuleType::Other, "sing.proxy"),
            rule(RuleType::Ip, "sing.proxy.ip"),
        ],
        vec![
            rule(RuleType::Process, "sing.reject"),
            rule(RuleType::Ip, "sing.reject.ip"),
        ],
    )
}

/// clash 使用的规则集合。
fn clash_rules() -> (Vec<Rule>, Vec<Rule>, Vec<Rule>) {
    (
        vec![
            rule(RuleType::Other, "clash.direct"),
            rule(RuleType::Ip, "clash.direct.ip"),
        ],
        vec![
            rule(RuleType::Other, "clash.proxy"),
            rule(RuleType::Ip, "clash.proxy.ip"),
        ],
        vec![
            rule(RuleType::Process, "clash.reject"),
            rule(RuleType::Ip, "clash.reject.ip"),
        ],
    )
}

/// 订阅来源。
enum Source {
    /// clash 订阅, 节点键为 clash 原生字段
    Clash(PickNodes),
    /// 纯净订阅, 节点键为分享链接参数
    Pure(PickNodes),
}

impl Source {
    /// 由解析出的节点识别来源, 依据协议所在的字段名。
    fn of(remote: &PickRemote) -> Self {
        let clash = remote
            .nodes
            .nodes
            .first()
            .is_some_and(|node| node.protocol_key == KEY_TYPE);

        let nodes = remote.nodes.clone();
        if clash {
            Self::Clash(nodes)
        } else {
            Self::Pure(nodes)
        }
    }
}

/// 从远程订阅获取并解析节点。
async fn fetch_remote(
    params: &ConvertParams,
    headers: MultiStringValue,
) -> Result<(PickRemote, Source, String)> {
    let url = &params.remote;
    console_debug!("从远程[{}]获取数据", url);

    let request = FetchRequest::new(url).with_headers(headers);
    let response = FetchClient::new().get(request).await?;
    if !response.is_success() {
        console_debug!("远程返回内容: {}", response.string());
        return Err(anyhow!("远程[{}]返回异常! {}", url, response.status));
    }

    let info = response.subscription_userinfo().cloned();
    let source = response.string();
    console_debug!("解析远程[{}]数据, 长度: {}", url, source.len());

    let remote = pick(&source, info.as_deref())?;
    let origin = Source::of(&remote);
    let filename = lib_core::url::pick_host(url).ok_or_else(|| anyhow!("invalid remote"))?;

    Ok((remote, origin, filename))
}

/// 构建平台配置。
fn config(params: &ConvertParams, rules: (Vec<Rule>, Vec<Rule>, Vec<Rule>)) -> PlatformConfig {
    let (rules_direct, rules_proxy, rules_reject) = rules;

    PlatformConfig {
        tun: params.tun,
        ipv6: params.ipv6,
        fake_ip: params.fake_ip,
        geo_cn: params.geo_cn,
        debug: params.debug,
        rules_direct,
        rules_proxy,
        rules_reject,
        include: params.include.clone(),
        exclude: params.exclude.clone(),
        ..PlatformConfig::default()
    }
}

/// 转换为 sing-box 配置。
pub async fn sing_box(params: &ConvertParams, headers: MultiStringValue) -> Result<Converted> {
    convert(params, headers, Platform::SingBox, sing_box_rules()).await
}

/// 转换为 clash 配置。
pub async fn clash(params: &ConvertParams, headers: MultiStringValue) -> Result<Converted> {
    convert(params, headers, Platform::Clash, clash_rules()).await
}

/// 获取订阅并按平台转换。
async fn convert(
    params: &ConvertParams,
    headers: MultiStringValue,
    platform: Platform,
    rules: (Vec<Rule>, Vec<Rule>, Vec<Rule>),
) -> Result<Converted> {
    let (remote, origin, filename) = fetch_remote(params, headers).await?;
    let info = remote.info();
    console_debug!("构造配置");

    let config = config(params, rules);
    let body = match origin {
        Source::Clash(nodes) => clash_to(platform, nodes, config)?,
        Source::Pure(nodes) => pure_to(platform, nodes, config)?,
    };
    console_debug!("配置转换完成");

    Ok(Converted {
        body,
        filename,
        info,
    })
}
