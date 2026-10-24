use std::sync::Arc;

use anyhow::Result;
use lib_core::next_id;
use lib_db::{DbContext, DbPool};
use types_core::dto::rule::{RuleRefreshDTO, RuleUpsertDTO};
use types_core::entity::rule::Rule;

use crate::repository::rule_repository as repository;

/// 规则表的业务入口。
pub struct RuleService {
    context: Arc<DbContext>,
}

impl RuleService {
    /// 绑定请求级数据库上下文。
    pub fn new(context: Arc<DbContext>) -> Self {
        Self { context }
    }

    /// 查询全部规则，按 id 倒序。
    pub async fn list(&self) -> Result<Vec<Rule>> {
        repository::all(self.pool()).await
    }

    /// 查询全部规则 id。
    pub async fn list_ids(&self) -> Result<Vec<i64>> {
        repository::all_id(self.pool()).await
    }

    /// 按 id 查询规则。
    pub async fn find(&self, id: i64) -> Result<Option<Rule>> {
        repository::find(self.pool(), id).await
    }

    /// 查询全部规则的刷新字段。
    pub async fn list_refresh(&self) -> Result<Vec<RuleRefreshDTO>> {
        repository::all_refresh(self.pool()).await
    }

    /// 按 id 查询规则的刷新字段。
    pub async fn find_refresh(&self, id: i64) -> Result<Option<RuleRefreshDTO>> {
        repository::find_refresh(self.pool(), id).await
    }

    /// 查询到达刷新时间的规则。
    pub async fn list_need_refresh(&self, millis: i64) -> Result<Vec<RuleRefreshDTO>> {
        repository::find_need_refresh(self.pool(), millis).await
    }

    /// 新增或修改规则，返回规则 id。
    ///
    /// 新增时补全 id、创建时间与刷新时间；修改时保留创建时间与刷新时间。
    pub async fn upsert(&self, dto: RuleUpsertDTO, millis: i64) -> Result<i64> {
        let Some(id) = dto.id else {
            let id = next_id()?;
            let entity = Rule {
                id,
                name: dto.name,
                url: dto.url,
                content: dto.content.unwrap_or_default(),
                interval: dto.interval,
                update_time: millis,
                create_time: millis,
                refresh_time: 0,
                count: 0,
                count_process: 0,
                count_ip: 0,
                count_other: 0,
            };
            repository::insert(self.pool(), &entity).await?;
            return Ok(id);
        };

        let current = repository::find(self.pool(), id).await?;
        let content = dto.content.unwrap_or_else(|| {
            current
                .as_ref()
                .map(|entity| entity.content.clone())
                .unwrap_or_default()
        });
        repository::update(
            self.pool(),
            id,
            &dto.name,
            &dto.url,
            &content,
            dto.interval,
            millis,
        )
        .await?;
        Ok(id)
    }

    /// 仅更新刷新时间。
    pub async fn touch_refresh(&self, id: i64, millis: i64) -> Result<()> {
        repository::update_refresh_time(self.pool(), id, millis).await
    }

    /// 写入刷新结果与规则数量统计，`content` 为 `None` 表示本地规则、不覆盖已保存的内容。
    #[allow(clippy::too_many_arguments)]
    pub async fn save_refresh(
        &self,
        id: i64,
        content: Option<&str>,
        refresh_time: i64,
        count: u64,
        count_process: u64,
        count_ip: u64,
        count_other: u64,
    ) -> Result<()> {
        repository::update_refresh(
            self.pool(),
            id,
            content,
            refresh_time,
            count,
            count_process,
            count_ip,
            count_other,
        )
        .await
    }

    /// 按 id 删除规则，返回删除行数。
    pub async fn delete(&self, id: i64) -> Result<u64> {
        repository::delete(self.pool(), id).await
    }

    fn pool(&self) -> &DbPool {
        self.context.pool()
    }
}
