use std::sync::Arc;

use anyhow::Result;
use lib_db::{DbContext, DbPool};
use types_core::entity::inner_rule::InnerRule;

use crate::repository::inner_rule_repository as repository;

/// 内核内置规则表的业务入口。
pub struct InnerRuleService {
    context: Arc<DbContext>,
}

impl InnerRuleService {
    /// 绑定请求级数据库上下文。
    pub fn new(context: Arc<DbContext>) -> Self {
        Self { context }
    }

    /// 查询全部未就绪的内置规则。
    pub async fn list_not_ready(&self) -> Result<Vec<InnerRule>> {
        repository::all_not_ready(self.pool()).await
    }

    /// 按 id 查询内置规则。
    pub async fn find(&self, id: &str) -> Result<Option<InnerRule>> {
        repository::find(self.pool(), id).await
    }

    /// 登记一条内置规则，已存在时保留已就绪状态。
    pub async fn register(&self, id: &str, url: &str) -> Result<()> {
        repository::insert_or_update(self.pool(), id, url).await
    }

    /// 标记为已就绪，并记录本次下载的 etag。
    pub async fn ready(&self, id: &str, etag: &str) -> Result<()> {
        repository::update_ready(self.pool(), id, etag).await
    }

    /// 标记为未就绪，下次启动前会重新下载。
    pub async fn not_ready(&self, id: &str) -> Result<()> {
        repository::update_not_ready(self.pool(), id).await
    }

    fn pool(&self) -> &DbPool {
        self.context.pool()
    }
}
