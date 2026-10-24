use anyhow::Result;
use lib_biz::rule::RuleBiz;
use lib_biz::rule_sync::RuleSync;
use service_core::service::rule_service::RuleService;
use types_core::dto::rule::RuleUpsertDTO;
use types_core::entity::rule::Rule;

use super::Services;

impl Services {
    /// 查询全部规则，按 id 倒序。
    pub async fn rule_list(&self) -> Result<Vec<Rule>> {
        RuleService::new(self.context()).list().await
    }
}

impl Services {
    /// 同步规则，未指定 id 时同步全部。
    ///
    /// 同步全部只负责推送，等待由后台任务继续完成。
    pub async fn sync_rule(&self, id: Option<&str>) -> Result<usize> {
        RuleBiz::new(self.context()).sync(id).await
    }

    /// 新增规则，写库后触发一次同步。
    pub async fn create_rule(&self, dto: RuleUpsertDTO) -> Result<String> {
        RuleBiz::new(self.context()).create(dto).await
    }

    /// 修改规则，地址变更后触发一次同步。
    pub async fn edit_rule(&self, dto: RuleUpsertDTO) -> Result<String> {
        RuleBiz::new(self.context()).edit(dto).await
    }

    /// 删除一条规则。
    pub async fn delete_rule(&self, id: &str) -> Result<()> {
        RuleBiz::new(self.context()).delete(id).await?;
        Ok(())
    }
}

impl Services {
    /// 把数据库上下文与运行时注入规则同步调度器。
    pub fn init_rule_sync(&self) {
        RuleSync::instance().init(self.context(), self.runtime_handle());
    }
}

impl Services {
    /// 正在刷新中的规则 id。
    ///
    /// 同步任务跑在后台调度器上，界面靠它区分「刷新中」与「已刷新」。
    pub fn rule_pending(&self) -> Vec<String> {
        RuleSync::instance().pending()
    }
}
