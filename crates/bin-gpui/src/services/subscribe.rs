use anyhow::Result;
use lib_core::current_millis;
use service_core::service::subscribe_service::SubscribeService;
use types_core::dto::subscribe::SubscribeUpsertDTO;
use types_core::entity::subscribe::Subscribe;

use super::Services;

impl Services {
    /// 查询全部订阅。
    pub async fn subscribe_list(&self) -> Result<Vec<Subscribe>> {
        SubscribeService::new(self.context()).list().await
    }
}

impl Services {
    /// 新增或修改订阅。
    ///
    /// 地址变更后已保存的远端内容不再对应当前地址，因此清空；本地订阅保留手写内容。
    pub async fn upsert_subscribe(&self, dto: SubscribeUpsertDTO) -> Result<String> {
        let service = SubscribeService::new(self.context());
        let content = match dto.id.as_deref().filter(|id| !id.is_empty()) {
            None => String::new(),
            Some(id) => match service.find(id).await? {
                Some(current) if current.url == dto.url => current.content,
                Some(_) => String::new(),
                None => String::new(),
            },
        };

        service
            .upsert(
                SubscribeUpsertDTO {
                    content: Some(content),
                    ..dto
                },
                current_millis()?,
            )
            .await
    }

    /// 删除一条订阅。
    pub async fn delete_subscribe(&self, id: &str) -> Result<()> {
        SubscribeService::new(self.context()).delete(id).await?;
        Ok(())
    }
}
