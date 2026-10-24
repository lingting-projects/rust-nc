use std::sync::Arc;

use anyhow::{Result, anyhow};
use lib_core::current_millis;
use lib_db::DbContext;
use log::debug;
use service_core::service::subscribe_service::SubscribeService;
use types_core::dto::subscribe::SubscribeUpsertDTO;

use crate::subscribe_sync::SubscribeSync;

/// 订阅的核心业务入口。
///
/// 负责订阅的新增与修改，并在需要时把订阅推送给 [`SubscribeSync`] 同步。
pub struct SubscribeBiz {
    context: Arc<DbContext>,
}

impl SubscribeBiz {
    /// 绑定请求级数据库上下文。
    pub fn new(context: Arc<DbContext>) -> Self {
        Self { context }
    }

    /// 同步订阅，返回新推入的任务数量。
    ///
    /// 未指定 `id` 时推送全部订阅并立即返回；指定 `id` 时推送该 id 并等待同步结束。
    pub async fn sync(&self, id: Option<i64>) -> Result<usize> {
        let sync = SubscribeSync::instance();
        let Some(id) = id else {
            return Ok(sync.push_all(self.service().list_ids().await?));
        };

        let pushed = usize::from(sync.push(id));
        sync.wait(id).await;
        Ok(pushed)
    }

    /// 新增订阅，写库后触发一次同步。
    pub async fn create(&self, dto: SubscribeUpsertDTO) -> Result<i64> {
        let id = self.service().upsert(dto, current_millis()?).await?;
        self.push(id);
        Ok(id)
    }

    /// 修改订阅，地址变更后触发一次同步。
    ///
    /// 地址变更后已保存的远端内容不再对应当前地址，因此清空；本地订阅保留手写内容。
    pub async fn edit(&self, dto: SubscribeUpsertDTO) -> Result<i64> {
        let id = existing_id(&dto)?;
        let service = self.service();
        let current = service.find(id).await?;
        let changed = current.as_ref().is_none_or(|item| item.url != dto.url);
        let content = match current {
            Some(item) if !changed => item.content,
            _ => String::new(),
        };

        let dto = SubscribeUpsertDTO {
            content: Some(content),
            ..dto
        };
        let id = service.upsert(dto, current_millis()?).await?;
        if changed {
            self.push(id);
        }
        Ok(id)
    }

    /// 触发同步但不等待结果；写库结果不受同步推送影响。
    fn push(&self, id: i64) {
        if !SubscribeSync::instance().push(id) {
            debug!("[订阅] [{}] 未推入同步队列", id);
        }
    }

    fn service(&self) -> SubscribeService {
        SubscribeService::new(self.context.clone())
    }
}

/// 取出修改入参中的有效 id。
fn existing_id(dto: &SubscribeUpsertDTO) -> Result<i64> {
    dto.id.ok_or_else(|| anyhow!("修改订阅缺少 id"))
}
