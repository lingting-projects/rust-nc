//! 内置规则的注册与可用性判定。
//!
//! 内置规则的名称与地址由 `service-core` 统一登记, 这里只负责:
//! 在需要时把内置规则登记到数据库并推入同步队列, 以及判定规则是否可用。
//!
//! 可用要求数据库 `ready` 为 1 且本地文件确实存在: 文件可能被外部删除或磁盘被清理,
//! 只信数据库会读到不存在的规则集。

use std::sync::Arc;

use anyhow::Result;
use lib_db::DbContext;
use service_core::service::inner_rule_service::InnerRuleService;
use types_core::entity::inner_rule::InnerRule;

use crate::inner_rule_file;
use crate::inner_rule_sync::InnerRuleSync;

/// 登记全部内置规则并推入同步队列, 返回成功入队的数量。
///
/// 已登记过的规则保持原状态, 因此可重复调用; 推送只负责入队,
/// 下载结果由同步调度器记录, 不阻塞调用方。
pub async fn register_all(context: &Arc<DbContext>) -> Result<usize> {
    let ids = InnerRuleService::new(context.clone())
        .register_all()
        .await?;
    Ok(InnerRuleSync::instance().push_all(ids))
}

/// 登记 geo cn 直连规则集并推入同步队列, 返回是否成功入队。
///
/// geo cn 直连是内核配置会引用的内置规则, 生成配置或启动内核前调用,
/// 保证规则集已在下载中。
pub async fn register_geo_cn(context: &Arc<DbContext>) -> Result<bool> {
    let service = InnerRuleService::new(context.clone());
    let id = service.register_geo_cn().await?;
    Ok(InnerRuleSync::instance().push(id))
}

/// 查询尚不可用的内置规则: 未就绪的, 以及已就绪但本地文件不存在的。
///
/// 可用要求数据库 `ready` 为 1 且本地文件存在; 文件缺失的规则先被置为未就绪,
/// 与未就绪的一起返回, 调用方统一重新下载。
pub async fn pending(context: &Arc<DbContext>) -> Result<Vec<InnerRule>> {
    let service = InnerRuleService::new(context.clone());
    let mut rules = reset_unavailable(context)
        .await?
        .into_iter()
        .chain(service.list_not_ready().await?)
        .collect::<Vec<_>>();
    rules.sort_by_key(|item| item.id);
    rules.dedup_by_key(|item| item.id);
    Ok(rules)
}

/// 查询可用状态与本地文件不一致的内置规则, 并把这些规则置为未就绪。
///
/// 数据库标记已就绪但文件不存在时, 只信数据库会读到不存在的规则集,
/// 因此先置为未就绪, 再由调用方重新下载; 已经未就绪的规则不重复处理。
async fn reset_unavailable(context: &Arc<DbContext>) -> Result<Vec<InnerRule>> {
    let service = InnerRuleService::new(context.clone());
    let mut unavailable = Vec::new();
    for rule in service.list_ready().await? {
        if inner_rule_file::inner_rule_file(&rule.name)?.exists() {
            continue;
        }

        log::warn!("[内置规则] [{}] 本地文件不存在, 标记为未就绪", rule.name);
        service.not_ready(rule.id).await?;
        unavailable.push(rule);
    }

    Ok(unavailable)
}
