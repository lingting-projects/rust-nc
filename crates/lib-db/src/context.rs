use std::future::Future;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use sqlx::SqlitePool;

/// 请求级数据库上下文，持有连接池。
pub struct DbContext {
    pool: SqlitePool,
}

impl DbContext {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 连接池。
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

tokio::task_local! {
    static DB_CONTEXT: Arc<DbContext>;
}

/// 在当前任务内注入数据库上下文。
pub async fn scope_db<F>(context: Arc<DbContext>, future: F) -> F::Output
where
    F: Future,
{
    DB_CONTEXT.scope(context, future).await
}

/// 取当前任务的数据库上下文。
pub fn use_db() -> Result<Arc<DbContext>> {
    DB_CONTEXT
        .try_with(Arc::clone)
        .map_err(|error| anyhow!("当前调用不在数据库上下文作用域内：{error}"))
}

/// 取当前任务的数据库连接池。
pub fn use_pool() -> Result<SqlitePool> {
    Ok(use_db()?.pool().clone())
}
