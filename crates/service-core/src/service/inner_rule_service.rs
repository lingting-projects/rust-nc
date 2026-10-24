use std::sync::Arc;

use anyhow::Result;
use lib_converter::constants::{GEO_IP_CN, INNER_RULE_GEO_IP_CN};
use lib_core::next_id;
use lib_db::{DbContext, DbPool};
use types_core::entity::inner_rule::InnerRule;

use crate::repository::inner_rule_repository as repository;

/// 全部内置规则: `(名称, 原始下载地址)`。
///
/// 名称同时是本地文件名与内核配置的引用键, 地址只保存原始地址,
/// 加速镜像在下载时按需拼接。
const BUILTIN_RULES: &[(&str, &str)] = &[(INNER_RULE_GEO_IP_CN, GEO_IP_CN)];

/// 内核内置规则表的业务入口。
pub struct InnerRuleService {
    context: Arc<DbContext>,
}

impl InnerRuleService {
    /// 绑定请求级数据库上下文。
    pub fn new(context: Arc<DbContext>) -> Self {
        Self { context }
    }

    /// 查询全部未就绪的内置规则。
    pub async fn list_not_ready(&self) -> Result<Vec<InnerRule>> {
        repository::all_not_ready(self.pool()).await
    }

    /// 查询全部已就绪的内置规则。
    pub async fn list_ready(&self) -> Result<Vec<InnerRule>> {
        repository::all_ready(self.pool()).await
    }

    /// 按 id 查询内置规则。
    pub async fn find(&self, id: i64) -> Result<Option<InnerRule>> {
        repository::find(self.pool(), id).await
    }

    /// 按名称登记一条内置规则，已存在时保留已就绪状态。
    ///
    /// 主键按登记顺序分配，名称是幂等键，重复登记只更新地址与就绪标记。
    pub async fn register(&self, name: &str, url: &str) -> Result<i64> {
        let id = next_id()?;
        repository::insert_or_update(self.pool(), id, name, url).await?;
        Ok(id)
    }

    /// 快捷登记全部内置规则, 返回全部规则 id。
    ///
    /// 名称与地址集中在本 Service, 调用方不必再持有内置规则清单;
    /// 重复登记保留已就绪状态, 因此可在任意启动路径上调用。
    pub async fn register_all(&self) -> Result<Vec<i64>> {
        self.register_builtin(BUILTIN_RULES).await
    }

    /// 快捷登记 geo cn 直连规则集, 返回规则 id。
    pub async fn register_geo_cn(&self) -> Result<i64> {
        self.register(INNER_RULE_GEO_IP_CN, GEO_IP_CN).await
    }

    /// 标记为已就绪，并记录本次下载的 etag。
    pub async fn ready(&self, id: i64, etag: &str) -> Result<()> {
        repository::update_ready(self.pool(), id, etag).await
    }

    /// 标记为未就绪，下次启动前会重新下载。
    pub async fn not_ready(&self, id: i64) -> Result<()> {
        repository::update_not_ready(self.pool(), id).await
    }

    /// 按清单逐条登记, 返回全部规则 id。
    async fn register_builtin(&self, rules: &[(&str, &str)]) -> Result<Vec<i64>> {
        let mut ids = Vec::with_capacity(rules.len());
        for (name, url) in rules {
            ids.push(self.register(name, url).await?);
        }

        Ok(ids)
    }

    fn pool(&self) -> &DbPool {
        self.context.pool()
    }
}
