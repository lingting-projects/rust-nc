//! `tbl_config` 表的业务操作。

use std::sync::Arc;

use anyhow::Result;
use lib_core::next_id;
use lib_db::{DbContext, DbPool};
use types_core::dto::config::ConfigUpsertDTO;
use types_core::entity::config::Config;

use crate::repository::config_repository as repository;

/// 内核配置表的业务入口。
pub struct ConfigService {
    context: Arc<DbContext>,
}

impl ConfigService {
    /// 绑定请求级数据库上下文。
    pub fn new(context: Arc<DbContext>) -> Self {
        Self { context }
    }

    /// 查询全部配置。
    pub async fn list(&self) -> Result<Vec<Config>> {
        repository::all(self.pool()).await
    }

    /// 按 id 查询配置。
    pub async fn find(&self, id: &str) -> Result<Option<Config>> {
        repository::find(self.pool(), id).await
    }

    /// 查询到达刷新时间的配置。
    pub async fn list_need_refresh(&self, millis: i64) -> Result<Vec<Config>> {
        repository::find_need_refresh(self.pool(), millis).await
    }

    /// 新增或修改配置，返回配置 id。
    ///
    /// 新增时补全 id 与时间字段；修改时保留创建时间与刷新时间。
    pub async fn upsert(&self, dto: ConfigUpsertDTO, millis: i64) -> Result<String> {
        let Some(id) = existing_id(&dto) else {
            let id = next_id()?.to_string();
            let entity = Config {
                id: id.clone(),
                name: dto.name,
                tun: dto.tun,
                fake_ip: dto.fake_ip,
                ipv6: dto.ipv6,
                subscribe_id: dto.subscribe_id,
                geo_cn: dto.geo_cn,
                rule_direct_ids: dto.rule_direct_ids,
                rule_proxy_ids: dto.rule_proxy_ids,
                rule_reject_ids: dto.rule_reject_ids,
                include_area_non: dto.include_area_non,
                include_area: dto.include_area,
                include_name_contains: dto.include_name_contains,
                exclude_area: dto.exclude_area,
                exclude_name_contains: dto.exclude_name_contains,
                interval: dto.interval,
                refresh_time: 0,
                update_time: millis,
                create_time: millis,
            };
            repository::insert(self.pool(), &entity).await?;
            return Ok(id);
        };

        let current = repository::find(self.pool(), &id).await?;
        let (refresh_time, create_time) = match current {
            Some(entity) => (entity.refresh_time, entity.create_time),
            None => (0, millis),
        };
        let entity = Config {
            id: id.clone(),
            name: dto.name,
            tun: dto.tun,
            fake_ip: dto.fake_ip,
            ipv6: dto.ipv6,
            subscribe_id: dto.subscribe_id,
            geo_cn: dto.geo_cn,
            rule_direct_ids: dto.rule_direct_ids,
            rule_proxy_ids: dto.rule_proxy_ids,
            rule_reject_ids: dto.rule_reject_ids,
            include_area_non: dto.include_area_non,
            include_area: dto.include_area,
            include_name_contains: dto.include_name_contains,
            exclude_area: dto.exclude_area,
            exclude_name_contains: dto.exclude_name_contains,
            interval: dto.interval,
            refresh_time,
            update_time: millis,
            create_time,
        };
        repository::update(self.pool(), &entity).await?;
        Ok(id)
    }

    /// 更新配置的刷新时间。
    pub async fn touch_refresh(&self, id: &str, millis: i64) -> Result<()> {
        repository::update_refresh_time(self.pool(), id, millis).await
    }

    /// 按 id 删除配置，返回删除行数。
    pub async fn delete(&self, id: &str) -> Result<u64> {
        repository::delete(self.pool(), id).await
    }

    fn pool(&self) -> &DbPool {
        self.context.pool()
    }
}

/// 取出入参中的有效 id，`None` 与空串都视为新增。
fn existing_id(dto: &ConfigUpsertDTO) -> Option<String> {
    match dto.id.as_ref() {
        Some(id) if !id.is_empty() => Some(id.clone()),
        _ => None,
    }
}
