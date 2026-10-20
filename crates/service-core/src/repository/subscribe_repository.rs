use std::str::FromStr;

use anyhow::Result;
use lib_db::{DbPool, DbRow};
use types_core::dto::subscribe::SubscribeRefreshDTO;
use types_core::entity::subscribe::{Subscribe, SubscribeSource};

use super::mapping;

/// 表名。
const TABLE: &str = "tbl_subscribe";

/// 列表查询字段，远端订阅不回传已保存的完整内容。
///
/// 节点单独用 [`find_nodes`] 查询：列表不需要节点，占位成空串，
/// 否则行映射读不到 `nodes` 列会整条查询失败。
const LIST_FIELDS: &str = "`id`,`name`,`url`,\
    CASE WHEN `url` LIKE 'http%' THEN '' ELSE `content` END AS `content`,\
    '' AS `nodes`,`source`,\
    `interval`,`update_time`,`create_time`,`refresh_time`,`download`,`upload`,`max`,`expire_time`";

/// 刷新字段，远端订阅不回传已保存的完整内容。
const REFRESH_FIELDS: &str = "`id`,`name`,`url`,\
    CASE WHEN `url` LIKE 'http%' THEN '' ELSE `content` END AS `content`";

/// 查询全部订阅, 按 id 倒序。
pub(crate) async fn all(pool: &DbPool) -> Result<Vec<Subscribe>> {
    let sql = format!("select {LIST_FIELDS} from {TABLE} order by `id` desc");
    let rows = sqlx::query(&sql).fetch_all(pool).await?;
    rows.iter().map(from_row).collect()
}

/// 查询全部订阅 id。
pub(crate) async fn all_id(pool: &DbPool) -> Result<Vec<String>> {
    let sql = format!("select `id` from {TABLE}");
    let rows = sqlx::query(&sql).fetch_all(pool).await?;
    rows.iter().map(|row| mapping::string(row, "id")).collect()
}

/// 按 id 查询订阅。
pub(crate) async fn find(pool: &DbPool, id: &str) -> Result<Option<Subscribe>> {
    let sql = format!("select {LIST_FIELDS} from {TABLE} where `id` = ? limit 1");
    let row = sqlx::query(&sql).bind(id).fetch_optional(pool).await?;
    row.as_ref().map(from_row).transpose()
}

/// 查询订阅解析后的节点 JSON 文本与源类型。
pub(crate) async fn find_nodes(pool: &DbPool, id: &str) -> Result<Option<(String, String)>> {
    let sql = format!("select `nodes`,`source` from {TABLE} where `id` = ? limit 1");
    let row = sqlx::query(&sql).bind(id).fetch_optional(pool).await?;
    let Some(row) = row else {
        return Ok(None);
    };
    Ok(Some((
        mapping::string(&row, "nodes")?,
        mapping::string(&row, "source")?,
    )))
}

/// 查询全部订阅的刷新字段。
pub(crate) async fn all_refresh(pool: &DbPool) -> Result<Vec<SubscribeRefreshDTO>> {
    let sql = format!("select {REFRESH_FIELDS} from {TABLE}");
    let rows = sqlx::query(&sql).fetch_all(pool).await?;
    rows.iter().map(from_refresh_row).collect()
}

/// 按 id 查询订阅的刷新字段。
pub(crate) async fn find_refresh(pool: &DbPool, id: &str) -> Result<Option<SubscribeRefreshDTO>> {
    let sql = format!("select {REFRESH_FIELDS} from {TABLE} where `id` = ? limit 1");
    let row = sqlx::query(&sql).bind(id).fetch_optional(pool).await?;
    row.as_ref().map(from_refresh_row).transpose()
}

/// 查询到达刷新时间的订阅。
pub(crate) async fn find_need_refresh(
    pool: &DbPool,
    millis: i64,
) -> Result<Vec<SubscribeRefreshDTO>> {
    let sql = format!(
        "select {REFRESH_FIELDS} from {TABLE} \
         where `interval` > 0 and `refresh_time` + `interval` <= ?"
    );
    let rows = sqlx::query(&sql).bind(millis).fetch_all(pool).await?;
    rows.iter().map(from_refresh_row).collect()
}

/// 新增订阅。
pub(crate) async fn insert(pool: &DbPool, entity: &Subscribe) -> Result<()> {
    let sql = format!(
        "insert into {TABLE} (`id`,`name`,`url`,`content`,`nodes`,`source`,`interval`,\
         `update_time`,`create_time`,`refresh_time`,`download`,`upload`,`max`,`expire_time`) \
         values (?,?,?,?,?,?,?,?,?,?,?,?,?,?)"
    );
    sqlx::query(&sql)
        .bind(&entity.id)
        .bind(&entity.name)
        .bind(&entity.url)
        .bind(&entity.content)
        .bind(&entity.nodes)
        .bind(entity.source.to_string())
        .bind(entity.interval)
        .bind(entity.update_time)
        .bind(entity.create_time)
        .bind(entity.refresh_time)
        .bind(entity.download as i64)
        .bind(entity.upload as i64)
        .bind(entity.max as i64)
        .bind(entity.expire_time)
        .execute(pool)
        .await?;
    Ok(())
}

/// 更新订阅的名称、地址、内容、间隔与更新时间。
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

/// 写入刷新结果，`content` 为 `None` 表示本地订阅、不覆盖内容。
#[allow(clippy::too_many_arguments)]
pub(crate) async fn update_refresh(
    pool: &DbPool,
    id: &str,
    content: Option<&str>,
    nodes: &str,
    source: &str,
    refresh_time: i64,
    download: u64,
    upload: u64,
    max: u64,
    expire_time: i64,
) -> Result<()> {
    let content_sql = if content.is_some() {
        "`content` = ?, "
    } else {
        ""
    };
    let sql = format!(
        "update {TABLE} set {content_sql}`nodes` = ?, `source` = ?, `refresh_time` = ?, `download` = ?, \
         `upload` = ?, `max` = ?, `expire_time` = ? where `id` = ?"
    );

    let mut query = sqlx::query(&sql);
    if let Some(content) = content {
        query = query.bind(content);
    }
    query
        .bind(nodes)
        .bind(source)
        .bind(refresh_time)
        .bind(download as i64)
        .bind(upload as i64)
        .bind(max as i64)
        .bind(expire_time)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// 按 id 删除订阅。
pub(crate) async fn delete(pool: &DbPool, id: &str) -> Result<u64> {
    let sql = format!("delete from {TABLE} where `id` = ?");
    let result = sqlx::query(&sql).bind(id).execute(pool).await?;
    Ok(result.rows_affected())
}

fn from_row(row: &DbRow) -> Result<Subscribe> {
    Ok(Subscribe {
        id: mapping::string(row, "id")?,
        name: mapping::string(row, "name")?,
        url: mapping::string(row, "url")?,
        content: mapping::string(row, "content")?,
        nodes: mapping::string(row, "nodes")?,
        // 库中文本无法识别时按未知处理
        source: SubscribeSource::from_str(&mapping::string(row, "source")?).unwrap_or_default(),
        interval: mapping::millis(row, "interval")?,
        update_time: mapping::millis(row, "update_time")?,
        create_time: mapping::millis(row, "create_time")?,
        refresh_time: mapping::millis(row, "refresh_time")?,
        download: mapping::count(row, "download")?,
        upload: mapping::count(row, "upload")?,
        max: mapping::count(row, "max")?,
        expire_time: mapping::millis(row, "expire_time")?,
    })
}

fn from_refresh_row(row: &DbRow) -> Result<SubscribeRefreshDTO> {
    Ok(SubscribeRefreshDTO {
        id: mapping::string(row, "id")?,
        name: mapping::string(row, "name")?,
        url: mapping::string(row, "url")?,
        content: mapping::string(row, "content")?,
    })
}
