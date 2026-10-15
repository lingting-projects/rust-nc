use anyhow::Result;
use lib_db::{DbPool, DbRow};
use types_core::dto::rule::RuleRefreshDTO;
use types_core::entity::rule::Rule;

use super::mapping;

/// 表名。
const TABLE: &str = "tbl_rule";

/// 列表查询字段，远端规则不回传已保存的完整内容。
const LIST_FIELDS: &str = "`id`,`name`,`url`,\
    CASE WHEN `url` LIKE 'http%' THEN '' ELSE `content` END AS `content`,\
    `interval`,`update_time`,`create_time`,`refresh_time`,\
    `count`,`count_process`,`count_ip`,`count_other`";

/// 刷新所需字段，远端规则不回传已保存的完整内容。
const REFRESH_FIELDS: &str = "`id`,`name`,`url`,\
    CASE WHEN `url` LIKE 'http%' THEN '' ELSE `content` END AS `content`";

/// 查询全部规则。
pub(crate) async fn all(pool: &DbPool) -> Result<Vec<Rule>> {
    let sql = format!("select {LIST_FIELDS} from {TABLE}");
    let rows = sqlx::query(&sql).fetch_all(pool).await?;
    rows.iter().map(from_row).collect()
}

/// 按 id 查询规则。
pub(crate) async fn find(pool: &DbPool, id: &str) -> Result<Option<Rule>> {
    let sql = format!("select {LIST_FIELDS} from {TABLE} where `id` = ? limit 1");
    let row = sqlx::query(&sql).bind(id).fetch_optional(pool).await?;
    row.as_ref().map(from_row).transpose()
}

/// 查询全部规则的刷新字段。
pub(crate) async fn all_refresh(pool: &DbPool) -> Result<Vec<RuleRefreshDTO>> {
    let sql = format!("select {REFRESH_FIELDS} from {TABLE}");
    let rows = sqlx::query(&sql).fetch_all(pool).await?;
    rows.iter().map(from_refresh_row).collect()
}

/// 按 id 查询规则的刷新字段。
pub(crate) async fn find_refresh(pool: &DbPool, id: &str) -> Result<Option<RuleRefreshDTO>> {
    let sql = format!("select {REFRESH_FIELDS} from {TABLE} where `id` = ? limit 1");
    let row = sqlx::query(&sql).bind(id).fetch_optional(pool).await?;
    row.as_ref().map(from_refresh_row).transpose()
}

/// 查询到达刷新时间的规则。
pub(crate) async fn find_need_refresh(pool: &DbPool, millis: i64) -> Result<Vec<RuleRefreshDTO>> {
    let sql = format!(
        "select {REFRESH_FIELDS} from {TABLE} \
         where `interval` > 0 and `refresh_time` + `interval` <= ?"
    );
    let rows = sqlx::query(&sql).bind(millis).fetch_all(pool).await?;
    rows.iter().map(from_refresh_row).collect()
}

/// 新增规则。
pub(crate) async fn insert(pool: &DbPool, entity: &Rule) -> Result<()> {
    let sql = format!(
        "insert into {TABLE} (`id`,`name`,`url`,`content`,`interval`,\
         `update_time`,`create_time`,`refresh_time`,\
         `count`,`count_process`,`count_ip`,`count_other`) \
         values (?,?,?,?,?,?,?,?,?,?,?,?)"
    );
    sqlx::query(&sql)
        .bind(&entity.id)
        .bind(&entity.name)
        .bind(&entity.url)
        .bind(&entity.content)
        .bind(entity.interval)
        .bind(entity.update_time)
        .bind(entity.create_time)
        .bind(entity.refresh_time)
        .bind(entity.count as i64)
        .bind(entity.count_process as i64)
        .bind(entity.count_ip as i64)
        .bind(entity.count_other as i64)
        .execute(pool)
        .await?;
    Ok(())
}

/// 更新规则的名称、地址、内容、间隔与更新时间。
pub(crate) async fn update(
    pool: &DbPool,
    id: &str,
    name: &str,
    url: &str,
    content: &str,
    interval: i64,
    update_time: i64,
) -> Result<()> {
    let sql = format!(
        "update {TABLE} set `name` = ?, `url` = ?, `content` = ?, `interval` = ?, \
         `update_time` = ? where `id` = ?"
    );
    sqlx::query(&sql)
        .bind(name)
        .bind(url)
        .bind(content)
        .bind(interval)
        .bind(update_time)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// 仅更新刷新时间，用于内容未变更的场景。
pub(crate) async fn update_refresh_time(pool: &DbPool, id: &str, millis: i64) -> Result<()> {
    let sql = format!("update {TABLE} set `refresh_time` = ? where `id` = ?");
    sqlx::query(&sql)
        .bind(millis)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// 写入刷新结果，`content` 为 `None` 表示本地规则、不覆盖内容。
#[allow(clippy::too_many_arguments)]
pub(crate) async fn update_refresh(
    pool: &DbPool,
    id: &str,
    content: Option<&str>,
    refresh_time: i64,
    count: u64,
    count_process: u64,
    count_ip: u64,
    count_other: u64,
) -> Result<()> {
    let content_sql = if content.is_some() {
        "`content` = ?, "
    } else {
        ""
    };
    let sql = format!(
        "update {TABLE} set {content_sql}`refresh_time` = ?, `count` = ?, \
         `count_process` = ?, `count_ip` = ?, `count_other` = ? where `id` = ?"
    );

    let mut query = sqlx::query(&sql);
    if let Some(content) = content {
        query = query.bind(content);
    }
    query
        .bind(refresh_time)
        .bind(count as i64)
        .bind(count_process as i64)
        .bind(count_ip as i64)
        .bind(count_other as i64)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// 按 id 删除规则。
pub(crate) async fn delete(pool: &DbPool, id: &str) -> Result<u64> {
    let sql = format!("delete from {TABLE} where `id` = ?");
    let result = sqlx::query(&sql).bind(id).execute(pool).await?;
    Ok(result.rows_affected())
}

fn from_row(row: &DbRow) -> Result<Rule> {
    Ok(Rule {
        id: mapping::string(row, "id")?,
        name: mapping::string(row, "name")?,
        url: mapping::string(row, "url")?,
        content: mapping::string(row, "content")?,
        interval: mapping::millis(row, "interval")?,
        update_time: mapping::millis(row, "update_time")?,
        create_time: mapping::millis(row, "create_time")?,
        refresh_time: mapping::millis(row, "refresh_time")?,
        count: mapping::count(row, "count")?,
        count_process: mapping::count(row, "count_process")?,
        count_ip: mapping::count(row, "count_ip")?,
        count_other: mapping::count(row, "count_other")?,
    })
}

fn from_refresh_row(row: &DbRow) -> Result<RuleRefreshDTO> {
    Ok(RuleRefreshDTO {
        id: mapping::string(row, "id")?,
        name: mapping::string(row, "name")?,
        url: mapping::string(row, "url")?,
        content: mapping::string(row, "content")?,
    })
}
