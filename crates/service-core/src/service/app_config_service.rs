use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use lib_db::DbContext;
use types_core::entity::setting::{AppConfig, AppConfigKey};

use crate::repository::app_config_repository as repository;

/// 键值配置的业务入口。
pub struct AppConfigService {
    context: Arc<DbContext>,
}

impl AppConfigService {
    /// 绑定请求级数据库上下文。
    pub fn new(context: Arc<DbContext>) -> Self {
        Self { context }
    }

    /// 读取配置，不存在时返回 `None`。
    pub async fn get(&self, key: &AppConfigKey) -> Result<Option<String>> {
        let entity = repository::find(self.context.pool(), &key.to_string()).await?;
        Ok(entity.map(|entity| entity.value))
    }

    /// 读取配置，不存在时返回默认值。
    pub async fn get_or(&self, key: &AppConfigKey, default: impl Into<String>) -> Result<String> {
        Ok(self.get(key).await?.unwrap_or_else(|| default.into()))
    }

    /// 批量读取配置，缺失的键不出现在结果中。
    pub async fn map(&self, keys: &[AppConfigKey]) -> Result<HashMap<String, String>> {
        let names = keys.iter().map(|key| key.to_string()).collect::<Vec<_>>();
        let entities = repository::find_by_keys(self.context.pool(), &names).await?;
        Ok(entities
            .into_iter()
            .map(|entity| (entity.key, entity.value))
            .collect())
    }

    /// 读取全部配置，按 key 组成映射。
    pub async fn all(&self) -> Result<HashMap<String, String>> {
        let entities = repository::find_all(self.context.pool()).await?;
        Ok(entities
            .into_iter()
            .map(|entity| (entity.key, entity.value))
            .collect())
    }

    /// 写入单条配置，已存在则覆盖。
    pub async fn set(&self, key: &AppConfigKey, value: &str) -> Result<()> {
        repository::upsert(self.context.pool(), &key.to_string(), value).await
    }

    /// 批量写入配置，单条 SQL 完成。
    pub async fn set_all(&self, items: &[(AppConfigKey, String)]) -> Result<()> {
        let entities = items
            .iter()
            .map(|(key, value)| AppConfig {
                key: key.to_string(),
                value: value.clone(),
            })
            .collect::<Vec<_>>();
        repository::upsert_all(self.context.pool(), &entities).await
    }
}
