use crate::{DatabaseOptions, DbPool};
use anyhow::{Context, Result};
use sqlx::Sqlite;
use sqlx::Transaction;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::path::Path;
use std::time::Duration;
use tracing::log;

/// 数据库文件名，位于 `DatabaseOptions::path` 指定的目录下。
const DATABASE_FILE_NAME: &str = "data.sqlite";
/// 等待连接释放的最长时间。
pub(crate) const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(60);
/// 写入锁等待超时，避免并发写入直接返回 SQLITE_BUSY。
const BUSY_TIMEOUT: Duration = Duration::from_secs(30);

pub struct Database {
    pub(crate) pool: DbPool,
}

impl Database {
    pub async fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::connect(DatabaseOptions::new(path.as_ref().to_path_buf())).await
    }

    pub async fn connect(options: DatabaseOptions) -> Result<Self> {
        tokio::fs::create_dir_all(&options.path)
            .await
            .with_context(|| format!("创建数据库目录失败: {}", options.path.display()))?;

        let connect_options = SqliteConnectOptions::new()
            .filename(options.path.join(DATABASE_FILE_NAME))
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(BUSY_TIMEOUT);

        let pool = SqlitePoolOptions::new()
            .max_connections(options.max_connections)
            .acquire_timeout(ACQUIRE_TIMEOUT)
            .idle_timeout(None)
            .max_lifetime(None)
            .connect_with(connect_options)
            .await?;

        sqlx::migrate!("../../migrations")
            .run(&pool)
            .await
            .context("执行 SQLite migration 失败")?;

        let database = Self { pool };
        Ok(database)
    }

    pub fn pool(&self) -> &DbPool {
        &self.pool
    }

    pub async fn transaction<F, E>(&self, f: F) -> Result<E>
    where
        F: AsyncFnOnce(&mut Transaction<'_, Sqlite>) -> Result<E>,
    {
        let mut tx = self.pool.begin().await?;

        match f(&mut tx).await {
            Ok(e) => {
                tx.commit().await?;
                Ok(e)
            }
            Err(e) => {
                log::error!("启用事务的SQL执行异常! {e:?}");
                tx.rollback().await?;
                Err(e)
            }
        }
    }
}
