use std::sync::Arc;

use anyhow::Result;
use lib_core::current_millis;
use lib_db::DbContext;
use log::{debug, warn};
use service_core::service::config_service::ConfigService;
use types_core::dto::config::ConfigUpsertDTO;

use crate::config_sync::{self, ConfigSync};
use crate::file;

/// 配置的核心业务入口。
///
/// 负责配置的写操作，并在写库后把配置交给 [`ConfigSync`] 校验。
pub struct ConfigBiz {
    context: Arc<DbContext>,
}

impl ConfigBiz {
    /// 绑定请求级数据库上下文。
    pub fn new(context: Arc<DbContext>) -> Self {
        Self { context }
    }

    /// 同步配置，返回同步的配置数量。
    ///
    /// 未指定 `id` 时只把全部配置推入队列并立即返回；指定 `id` 时等待该配置同步结束。
    pub async fn sync(&self, id: Option<&str>) -> Result<usize> {
        let Some(id) = id else {
            let ids = self.service().list_ids().await?;
            return Ok(ConfigSync::instance().push_all(ids));
        };

        config_sync::sync(&self.context, id).await?;
        Ok(1)
    }

    /// 新增配置，写库后触发一次同步。
    pub async fn create(&self, dto: ConfigUpsertDTO) -> Result<String> {
        let id = self.service().create(dto, current_millis()?).await?;
        self.push(&id);
        Ok(id)
    }

    /// 修改配置，写库后触发一次同步。
    pub async fn edit(&self, dto: ConfigUpsertDTO) -> Result<String> {
        let id = self.service().edit(dto, current_millis()?).await?;
        self.push(&id);
        Ok(id)
    }

    /// 删除配置，返回删除行数，并清理该配置的文件目录。
    pub async fn delete(&self, id: &str) -> Result<u64> {
        let count = self.service().delete(id).await?;
        if let Err(error) = file::remove_config(id) {
            warn!("[配置] [{}] 清理配置文件失败: {error:#}", id);
        }
        Ok(count)
    }

    /// 触发同步但不等待结果；写库结果不受同步推送影响。
    fn push(&self, id: &str) {
        if !ConfigSync::instance().push(id) {
            debug!("[配置] [{}] 未推入同步队列", id);
        }
    }

    fn service(&self) -> ConfigService {
        ConfigService::new(self.context.clone())
    }
}
