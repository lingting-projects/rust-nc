use anyhow::Result;
use lib_db::{DbPool, DbRow};
use sqlx::Row;
use types_core::entity::setting::AppConfig;

use super::mapping;

/// 表名。
const TABLE: &str = "app_config";

/// 按键查询单条配置。
pub(crate) async fn find(pool: &DbPool, key: &str) -> Result<Option<AppConfig>> {
    let sql = format!("select `key`, `value` from {TABLE} where `key` = ? limit 1");
    let row = sqlx::query(&sql).bind(key).fetch_optional(pool).await?;
    row.as_ref().map(from_row).transpose()
}

/// 按 key 集合批量查询。
pub(crate) async fn find_by_keys(pool: &DbPool, keys: &[String]) -> Result<Vec<AppConfig>> {
    if keys.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders = keys.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!("select `key`, `value` from {TABLE} where `key` in ({placeholders})");
    let mut query = sqlx::query(&sql);
    for key in keys {
        query = query.bind(key);
    }

    let rows = query.fetch_all(pool).await?;
    rows.iter().map(from_row).collect()
}

/// 查询全部配置。
pub(crate) async fn find_all(pool: &DbPool) -> Result<Vec<AppConfig>> {
    let sql = format!("select `key`, `value` from {TABLE}");
    let rows = sqlx::query(&sql).fetch_all(pool).await?;
    rows.iter().map(from_row).collect()
}

/// 写入配置，已存在则覆盖。
pub(crate) async fn upsert(pool: &DbPool, key: &str, value: &str) -> Result<()> {
    let sql = format!("replace into {TABLE} (`key`, `value`) values (?, ?)");
    sqlx::query(&sql)
        .bind(key)
        .bind(value)
        .execute(pool)
        .await?;
    Ok(())
}

/// 批量写入配置，单条 SQL 完成。
pub(crate) async fn upsert_all(pool: &DbPool, items: &[AppConfig]) -> Result<()> {
    if items.is_empty() {
        return Ok(());
    }

    let placeholders = items.iter().map(|_| "(?, ?)").collect::<Vec<_>>().join(",");
    let sql = format!("replace into {TABLE} (`key`, `value`) values {placeholders}");
    let mut query = sqlx::query(&sql);
    for item in items {
        query = query.bind(&item.key).bind(&item.value);
    }
    query.execute(pool).await?;
    Ok(())
}

fn from_row(row: &DbRow) -> Result<AppConfig> {
    Ok(AppConfig {
        key: row.try_get("key")?,
        value: mapping::string(row, "value")?,
    })
}
