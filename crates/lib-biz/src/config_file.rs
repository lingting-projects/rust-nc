use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Result, anyhow, bail};
use lib_converter::{NodeContains, Platform, PlatformConfig, Rule, RuleType, clash_to, pure_to};
use lib_core::directorys;
use lib_db::DbContext;
use lib_pick::{KEY_SCHEME, KEY_TYPE, PickNodes};
use service_core::manager::setting_manager::SettingManager;
use service_core::service::rule_service::RuleService;
use service_core::service::subscribe_service::SubscribeService;
use types_core::dto::setting::SettingKernel;
use types_core::entity::config::Config;

use crate::file;

/// 生成配置的 sing-box 配置文件: `{config}/{id}/sing_box/config.json`。
///
/// 引用的规则会复制到同一目录, 配置目录因此自包含。
pub async fn write(context: &Arc<DbContext>, config: &Config) -> Result<PathBuf> {
    let nodes = nodes(context, &config.subscribe_id).await?;
    let kernel = kernel(context).await?;
    // 先清空上一次的规则副本, 避免规则变更后残留旧文件
    file::remove_config(&config.id)?;
    let rules = rules(context, config).await?;

    let platform = PlatformConfig {
        ui_listen: kernel.ui_listen,
        ui_port: kernel.ui_port,
        mixed_listen: kernel.mixed_listen,
        mixed_port: kernel.mixed_port,
        tun: tun(config),
        fake_ip: config.fake_ip,
        ipv6: config.ipv6,
        geo_cn: config.geo_cn,
        debug: false,
        log_path: file::sing_box_log()?,
        rules_direct: rules.0,
        rules_proxy: rules.1,
        rules_reject: rules.2,
        dns_cn: kernel.dns_cn,
        dns_proxy: kernel.dns_proxy,
        include: include(config),
        exclude: exclude(config),
    };

    let id = config.id.clone();
    // 转换与写盘为同步计算, 放到阻塞线程池, 避免占住异步线程
    tokio::task::spawn_blocking(move || {
        let converted = if is_clash(&nodes) {
            clash_to(Platform::SingBox, nodes, platform)
        } else {
            pure_to(Platform::SingBox, nodes, platform)
        }?;

        let path = file::sing_box_json(&id)?;
        file::write(&path, converted)?;
        Ok(path)
    })
    .await?
}

/// 读取订阅已同步的节点。
async fn nodes(context: &Arc<DbContext>, id: &str) -> Result<PickNodes> {
    let nodes = SubscribeService::new(context.clone())
        .find_nodes(id)
        .await?
        .unwrap_or_default();
    if nodes.trim().is_empty() {
        bail!("订阅尚未同步出节点");
    }

    // 节点 JSON 的协议字段随订阅来源变化, 两种键都尝试
    PickNodes::from_json(&nodes, KEY_TYPE).or_else(|_| PickNodes::from_json(&nodes, KEY_SCHEME))
}

/// 读取内核设置。
async fn kernel(context: &Arc<DbContext>) -> Result<SettingKernel> {
    let cache = directorys()?.cache.clone();
    SettingManager::new(context.clone(), cache).kernel().await
}

/// 按直连、代理、拒绝的顺序取得规则。
async fn rules(
    context: &Arc<DbContext>,
    config: &Config,
) -> Result<(Vec<Rule>, Vec<Rule>, Vec<Rule>)> {
    let exist = RuleService::new(context.clone()).list_ids().await?;
    let id = &config.id;

    Ok((
        collect(id, &exist, &config.rule_direct_ids)?,
        collect(id, &exist, &config.rule_proxy_ids)?,
        collect(id, &exist, &config.rule_reject_ids)?,
    ))
}

/// 取得规则 id 对应的本地规则集, 并把规则文件复制到配置的 sing_box 目录。
///
/// 每个规则按类型各有 json 与 srs 两份产物, 存在 srs 时优先使用;
/// 复制后配置目录自包含, 规则更新后由下一次配置同步刷新副本。
fn collect(config_id: &str, exist: &[String], ids: &[String]) -> Result<Vec<Rule>> {
    let root = file::sing_box_dir(config_id)?;
    let mut rules = Vec::new();
    for id in ids {
        if !exist.iter().any(|item| item == id) {
            log::warn!("[配置] [{}] 规则不存在, 跳过: {id}", config_id);
            continue;
        }

        for rule_type in [RuleType::Process, RuleType::Other, RuleType::Ip] {
            let name = rule_type.name();
            let srs = file::rule_file(id, name, "srs")?;
            let json = file::rule_file(id, name, "json")?;
            let (source, ext) = match srs.exists() {
                true => (srs, "srs"),
                false if json.exists() => (json, "json"),
                false => continue,
            };

            // 不同规则可能产生同类型文件, 文件名带上规则 id 以免互相覆盖
            let target = root.join(format!("rule_{}_{}.{}", id, name, ext));
            file::copy(&source, &target)?;

            let path = target
                .to_str()
                .ok_or_else(|| anyhow!("规则文件路径非 UTF-8: {}", target.display()))?;
            rules.push(Rule::from_local(rule_type, path.to_string()));
        }
    }

    Ok(rules)
}

/// 由配置的包含条件构建节点筛选条件。
fn include(config: &Config) -> NodeContains {
    NodeContains {
        area: config.include_area.clone(),
        name_contains: config.include_name_contains.clone(),
        non_area: config.include_area_non,
        non_name: NodeContains::default_include().non_name,
    }
}

/// 由配置的排除条件构建节点筛选条件。
fn exclude(config: &Config) -> NodeContains {
    let default = NodeContains::default_exclude();
    NodeContains {
        area: config.exclude_area.clone(),
        name_contains: config.exclude_name_contains.clone(),
        non_area: default.non_area,
        non_name: default.non_name,
    }
}

/// 是否启用 tun。
///
/// 开发构建不会以管理员权限运行，因此 debug 下无条件关闭 tun，忽略配置中的取值。
fn tun(config: &Config) -> bool {
    if cfg!(debug_assertions) {
        return false;
    }
    config.tun
}

/// 由节点识别订阅来源, 依据协议所在的字段名。
fn is_clash(nodes: &PickNodes) -> bool {
    nodes
        .nodes
        .first()
        .is_some_and(|node| node.protocol_key == KEY_TYPE)
}
