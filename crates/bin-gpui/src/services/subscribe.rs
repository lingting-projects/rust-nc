use anyhow::Result;
use lib_biz::subscribe::SubscribeBiz;
use lib_biz::subscribe_sync::SubscribeSync;
use service_core::service::subscribe_service::SubscribeService;
use types_core::dto::subscribe::SubscribeUpsertDTO;
use types_core::entity::subscribe::Subscribe;

use super::Services;

impl Services {
    /// 查询全部订阅，按 id 倒序。
    pub async fn subscribe_list(&self) -> Result<Vec<Subscribe>> {
        SubscribeService::new(self.context()).list().await
    }
}

impl Services {
    /// 同步订阅，未指定 id 时同步全部。
    ///
    /// 同步全部只负责推送，等待由后台任务继续完成。
    pub async fn sync_subscribe(&self, id: Option<i64>) -> Result<usize> {
        SubscribeBiz::new(self.context()).sync(id).await
    }

    /// 新增订阅，写库后触发一次同步。
    pub async fn create_subscribe(&self, dto: SubscribeUpsertDTO) -> Result<i64> {
        SubscribeBiz::new(self.context()).create(dto).await
    }

    /// 修改订阅，地址变更后触发一次同步。
    pub async fn edit_subscribe(&self, dto: SubscribeUpsertDTO) -> Result<i64> {
        SubscribeBiz::new(self.context()).edit(dto).await
    }

    /// 删除一条订阅。
    ///
    /// 走业务层以便删除后发布数据更新事件，页面据此刷新。
    pub async fn delete_subscribe(&self, id: i64) -> Result<()> {
        SubscribeBiz::new(self.context()).delete(id).await?;
        Ok(())
    }
}

impl Services {
    /// 把数据库上下文与运行时注入订阅同步调度器。
    ///
    /// 调度器在任务提交时才使用依赖，因此未初始化时只会积压队列。
    pub fn init_subscribe_sync(&self) {
        SubscribeSync::instance().init(self.context(), self.runtime_handle());
    }
}

impl Services {
    /// 正在刷新中的订阅 id。
    ///
    /// 同步任务跑在后台调度器上，界面靠它区分「刷新中」与「已刷新」。
    pub fn subscribe_pending(&self) -> Vec<i64> {
        SubscribeSync::instance().pending()
    }
}
