use std::sync::Arc;

use anyhow::Result;
use lib_core::next_id;
use lib_db::DbContext;
use types_core::dto::subscribe::{SubscribeRefreshDTO, SubscribeUpsertDTO};
use types_core::entity::subscribe::Subscribe;

use crate::repository::subscribe_repository as repository;

/// 订阅表的业务入口。
pub struct SubscribeService {
    context: Arc<DbContext>,
}

impl SubscribeService {
    /// 绑定请求级数据库上下文。
    pub fn new(context: Arc<DbContext>) -> Self {
        Self { context }
    }

    /// 查询全部订阅。
    pub async fn list(&self) -> Result<Vec<Subscribe>> {
        repository::all(self.pool()).await
    }

    /// 按 id 查询订阅。
    pub async fn find(&self, id: &str) -> Result<Option<Subscribe>> {
        repository::find(self.pool(), id).await
    }

    /// 查询订阅解析后的节点 JSON 文本。
    pub async fn find_nodes(&self, id: &str) -> Result<Option<String>> {
        repository::find_nodes(self.pool(), id).await
    }

    /// 查询全部订阅的刷新字段。
    pub async fn list_refresh(&self) -> Result<Vec<SubscribeRefreshDTO>> {
        repository::all_refresh(self.pool()).await
    }

    /// 按 id 查询订阅的刷新字段。
    pub async fn find_refresh(&self, id: &str) -> Result<Option<SubscribeRefreshDTO>> {
        repository::find_refresh(self.pool(), id).await
    }

    /// 查询到达刷新时间的订阅。
    pub async fn list_need_refresh(&self, millis: i64) -> Result<Vec<SubscribeRefreshDTO>> {
        repository::find_need_refresh(self.pool(), millis).await
    }

    /// 新增或修改订阅，返回订阅 id。
    ///
    /// 新增时补全 id、创建时间与刷新时间；修改时保留创建时间与刷新时间。
    pub async fn upsert(&self, dto: SubscribeUpsertDTO, millis: i64) -> Result<String> {
        let Some(id) = existing_id(&dto) else {
            let id = next_id()?.to_string();
            let entity = Subscribe {
                id: id.clone(),
                name: dto.name,
                url: dto.url,
                content: dto.content.unwrap_or_default(),
                nodes: String::new(),
                interval: dto.interval,
                update_time: millis,
                create_time: millis,
                refresh_time: 0,
                download: 0,
                upload: 0,
                max: 0,
                expire_time: 0,
            };
            repository::insert(self.pool(), &entity).await?;
            return Ok(id);
        };

        let current = repository::find(self.pool(), &id).await?;
        let content = dto.content.unwrap_or_else(|| {
            current
                .as_ref()
                .map(|entity| entity.content.clone())
                .unwrap_or_default()
        });
        repository::update(
            self.pool(),
            &id,
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
    pub async fn touch_refresh(&self, id: &str, millis: i64) -> Result<()> {
        repository::update_refresh_time(self.pool(), id, millis).await
    }

    /// 写入刷新结果，`content` 为 `None` 表示本地订阅、不覆盖已保存的内容。
    #[allow(clippy::too_many_arguments)]
    pub async fn save_refresh(
        &self,
        id: &str,
        content: Option<&str>,
        nodes: &str,
        refresh_time: i64,
        download: u64,
        upload: u64,
        max: u64,
        expire_time: i64,
    ) -> Result<()> {
        repository::update_refresh(
            self.pool(),
            id,
            content,
            nodes,
            refresh_time,
            download,
            upload,
            max,
            expire_time,
        )
        .await
    }

    /// 按 id 删除订阅，返回删除行数。
    pub async fn delete(&self, id: &str) -> Result<u64> {
        repository::delete(self.pool(), id).await
    }

    fn pool(&self) -> &lib_db::DbPool {
        self.context.pool()
    }
}

/// 取出入参中的有效 id，`None` 与空串都视为新增。
fn existing_id(dto: &SubscribeUpsertDTO) -> Option<String> {
    match dto.id.as_ref() {
        Some(id) if !id.is_empty() => Some(id.clone()),
        _ => None,
    }
}
