use std::sync::Arc;

use anyhow::{Result, anyhow};
use lib_core::current_millis;
use lib_db::DbContext;
use log::{debug, warn};
use service_core::service::rule_service::RuleService;
use types_core::dto::rule::RuleUpsertDTO;

use crate::file;
use crate::rule_sync::RuleSync;

/// 规则的核心业务入口。
///
/// 负责规则的新增、修改与删除，并在需要时把规则推送给 [`RuleSync`] 同步。
pub struct RuleBiz {
    context: Arc<DbContext>,
}

impl RuleBiz {
    /// 绑定请求级数据库上下文。
    pub fn new(context: Arc<DbContext>) -> Self {
        Self { context }
    }

    /// 同步规则，返回新推入的任务数量。
    ///
    /// 未指定 `id` 时推送全部规则并立即返回；指定 `id` 时推送该 id 并等待同步结束。
    pub async fn sync(&self, id: Option<&str>) -> Result<usize> {
        let sync = RuleSync::instance();
        let Some(id) = id else {
            return Ok(sync.push_all(self.service().list_ids().await?));
        };

        let pushed = usize::from(sync.push(id));
        sync.wait(id).await;
        Ok(pushed)
    }

    /// 新增规则，写库后触发一次同步。
    pub async fn create(&self, dto: RuleUpsertDTO) -> Result<String> {
        let id = self.service().upsert(dto, current_millis()?).await?;
        self.push(&id);
        Ok(id)
    }

    /// 修改规则，地址变更后触发一次同步。
    ///
    /// 地址变更后已保存的远端内容不再对应当前地址，因此清空；
    /// 地址未变更时沿用调用方提交的内容（本地规则即手写内容），未提交则保留原内容。
    pub async fn edit(&self, dto: RuleUpsertDTO) -> Result<String> {
        let id = existing_id(&dto)?;
        let service = self.service();
        let current = service.find(&id).await?;
        let changed = current.as_ref().is_none_or(|item| item.url != dto.url);
        let content = match (&dto.content, &current) {
            (Some(content), _) => Some(content.clone()),
            (None, _) if changed => Some(String::new()),
            (None, Some(item)) => Some(item.content.clone()),
            (None, None) => Some(String::new()),
        };

        let dto = RuleUpsertDTO { content, ..dto };
        let id = service.upsert(dto, current_millis()?).await?;
        if changed {
            self.push(&id);
        }
        Ok(id)
    }

    /// 删除规则，返回删除行数，并清理该规则的规则文件目录。
    pub async fn delete(&self, id: &str) -> Result<u64> {
        let count = self.service().delete(id).await?;
        if let Err(error) = file::remove_rule(id) {
            warn!("[规则] [{}] 清理规则文件失败: {error:#}", id);
        }
        Ok(count)
    }

    /// 触发同步但不等待结果；写库结果不受同步推送影响。
    fn push(&self, id: &str) {
        if !RuleSync::instance().push(id) {
            debug!("[规则] [{}] 未推入同步队列", id);
        }
    }

    fn service(&self) -> RuleService {
        RuleService::new(self.context.clone())
    }
}

/// 取出修改入参中的有效 id。
fn existing_id(dto: &RuleUpsertDTO) -> Result<String> {
    match dto.id.as_ref().filter(|id| !id.is_empty()) {
        Some(id) => Ok(id.clone()),
        None => Err(anyhow!("修改规则缺少 id")),
    }
}
