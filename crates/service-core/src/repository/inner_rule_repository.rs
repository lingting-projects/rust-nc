use anyhow::Result;
use lib_db::{DbPool, DbRow};
use types_core::entity::inner_rule::InnerRule;

use super::mapping;

/// 表名。
const TABLE: &str = "tbl2_inner_rule";

/// 查询字段，与 [`InnerRule`] 字段一一对应。
const FIELDS: &str = "`id`,`name`,`url`,`etag`,`ready`";

/// 查询全部未就绪的内置规则。
pub(crate) async fn all_not_ready(pool: &DbPool) -> Result<Vec<InnerRule>> {
    let sql = format!("select {FIELDS} from {TABLE} where `ready` = 0 order by `id`");
    let rows = sqlx::query(&sql).fetch_all(pool).await?;
    rows.iter().map(from_row).collect()
}

/// 查询全部已就绪的内置规则。
pub(crate) async fn all_ready(pool: &DbPool) -> Result<Vec<InnerRule>> {
    let sql = format!("select {FIELDS} from {TABLE} where `ready` = 1 order by `id`");
    let rows = sqlx::query(&sql).fetch_all(pool).await?;
    rows.iter().map(from_row).collect()
}

/// 按 id 查询内置规则。
pub(crate) async fn find(pool: &DbPool, id: i64) -> Result<Option<InnerRule>> {
    let sql = format!("select {FIELDS} from {TABLE} where `id` = ? limit 1");
    let row = sqlx::query(&sql).bind(id).fetch_optional(pool).await?;
    row.as_ref().map(from_row).transpose()
}

/// 按名称登记内置规则，已存在时保留 `etag` 与 `ready`。
///
/// 地址变化说明本地文件不再对应当前内容，此时把 `ready` 重置为 0。
pub(crate) async fn insert_or_update(pool: &DbPool, id: i64, name: &str, url: &str) -> Result<()> {
    let sql = format!(
        "insert into {TABLE} (`id`,`name`,`url`,`etag`,`ready`) values (?,?,?,'',0) \
         on conflict(`name`) do update set \
         `ready` = case when `url` = excluded.`url` then `ready` else 0 end, \
         `etag` = case when `url` = excluded.`url` then `etag` else '' end, \
         `url` = excluded.`url`"
    );
    sqlx::query(&sql)
        .bind(id)
        .bind(name)
        .bind(url)
        .execute(pool)
        .await?;
    Ok(())
}

/// 标记为已就绪，并记录本次下载的 etag。
pub(crate) async fn update_ready(pool: &DbPool, id: i64, etag: &str) -> Result<()> {
    let sql = format!("update {TABLE} set `ready` = 1, `etag` = ? where `id` = ?");
    sqlx::query(&sql).bind(etag).bind(id).execute(pool).await?;
    Ok(())
}

/// 标记为未就绪。
pub(crate) async fn update_not_ready(pool: &DbPool, id: i64) -> Result<()> {
    let sql = format!("update {TABLE} set `ready` = 0 where `id` = ?");
    sqlx::query(&sql).bind(id).execute(pool).await?;
    Ok(())
}

fn from_row(row: &DbRow) -> Result<InnerRule> {
    Ok(InnerRule {
        id: mapping::id(row, "id")?,
        name: mapping::string(row, "name")?,
        url: mapping::string(row, "url")?,
        etag: mapping::string(row, "etag")?,
        ready: mapping::boolean(row, "ready")?,
    })
}
