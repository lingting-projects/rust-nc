//! 数据库行到领域类型的字段映射辅助。

use anyhow::{Context, Result};
use lib_db::DbRow;
use serde::de::DeserializeOwned;
use sqlx::Row;

/// 读取文本字段，`NULL` 视为空字符串。
pub(crate) fn string(row: &DbRow, name: &str) -> Result<String> {
    let value = row.try_get::<Option<String>, _>(name)?;
    Ok(value.unwrap_or_default())
}

/// 读取布尔字段，`NULL` 视为 `false`。
pub(crate) fn boolean(row: &DbRow, name: &str) -> Result<bool> {
    let value = row.try_get::<Option<bool>, _>(name)?;
    Ok(value.unwrap_or(false))
}

/// 读取毫秒级时间戳，`NULL` 视为 `0`。
pub(crate) fn millis(row: &DbRow, name: &str) -> Result<i64> {
    let value = row.try_get::<Option<i64>, _>(name)?;
    Ok(value.unwrap_or(0))
}

/// 读取计数，`NULL` 视为 `0`。
pub(crate) fn count(row: &DbRow, name: &str) -> Result<u64> {
    let value = row.try_get::<Option<i64>, _>(name)?;
    Ok(value.unwrap_or(0).max(0) as u64)
}

/// 读取 JSON 数组文本并反序列化，`NULL` 与空文本视为空集合。
pub(crate) fn json_vec<T>(row: &DbRow, name: &str) -> Result<Vec<T>>
where
    T: DeserializeOwned,
{
    let text = string(row, name)?;
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }

    let value = serde_json::from_str::<Vec<T>>(&text)
        .with_context(|| format!("字段 {name} 不是合法的 JSON 数组: {text}"))?;
    Ok(value)
}
