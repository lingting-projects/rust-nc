use anyhow::Result;
use lib_db::{DbPool, DbRow};
use types_core::entity::config::Config;

use super::mapping;

/// 表名。
const TABLE: &str = "tbl_config";

/// 查询字段，与 [`Config`] 字段一一对应。
const FIELDS: &str = "`id`,`name`,`tun`,`fake_ip`,`ipv6`,`subscribe_id`,`geo_cn`,\
    `rule_direct_ids`,`rule_proxy_ids`,`rule_reject_ids`,\
    `include_area_non`,`include_area`,`include_name_contains`,\
    `exclude_area`,`exclude_name_contains`,\
    `interval`,`refresh_time`,`update_time`,`create_time`";

/// 查询全部配置。
pub(crate) async fn all(pool: &DbPool) -> Result<Vec<Config>> {
    let sql = format!("select {FIELDS} from {TABLE}");
    let rows = sqlx::query(&sql).fetch_all(pool).await?;
    rows.iter().map(from_row).collect()
}

/// 按 id 查询配置。
pub(crate) async fn find(pool: &DbPool, id: &str) -> Result<Option<Config>> {
    let sql = format!("select {FIELDS} from {TABLE} where `id` = ? limit 1");
    let row = sqlx::query(&sql).bind(id).fetch_optional(pool).await?;
    row.as_ref().map(from_row).transpose()
}

/// 查询到达刷新时间的配置。
pub(crate) async fn find_need_refresh(pool: &DbPool, millis: i64) -> Result<Vec<Config>> {
    let sql = format!(
        "select {FIELDS} from {TABLE} \
         where `interval` > 0 and `refresh_time` + `interval` <= ?"
    );
    let rows = sqlx::query(&sql).bind(millis).fetch_all(pool).await?;
    rows.iter().map(from_row).collect()
}

/// 新增配置。
pub(crate) async fn insert(pool: &DbPool, entity: &Config) -> Result<()> {
    let sql = format!(
        "insert into {TABLE} (`id`,`name`,`tun`,`fake_ip`,`ipv6`,\
         `subscribe_id`,`geo_cn`,`rule_direct_ids`,`rule_proxy_ids`,`rule_reject_ids`,\
         `include_area_non`,`include_area`,`include_name_contains`,\
         `exclude_area`,`exclude_name_contains`,\
         `interval`,`refresh_time`,`update_time`,`create_time`) \
         values (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"
    );
    sqlx::query(&sql)
        .bind(&entity.id)
        .bind(&entity.name)
        .bind(entity.tun)
        .bind(entity.fake_ip)
        .bind(entity.ipv6)
        .bind(&entity.subscribe_id)
        .bind(entity.geo_cn)
        .bind(to_json(&entity.rule_direct_ids)?)
        .bind(to_json(&entity.rule_proxy_ids)?)
        .bind(to_json(&entity.rule_reject_ids)?)
        .bind(entity.include_area_non)
        .bind(to_json(&entity.include_area)?)
        .bind(to_json(&entity.include_name_contains)?)
        .bind(to_json(&entity.exclude_area)?)
        .bind(to_json(&entity.exclude_name_contains)?)
        .bind(entity.interval)
        .bind(entity.refresh_time)
        .bind(entity.update_time)
        .bind(entity.create_time)
        .execute(pool)
        .await?;
    Ok(())
}

/// 更新配置，刷新时间与创建时间保持不变。
#[allow(clippy::too_many_arguments)]
pub(crate) async fn update(pool: &DbPool, entity: &Config) -> Result<()> {
    let sql = format!(
        "update {TABLE} set `name` = ?, `tun` = ?, `fake_ip` = ?, `ipv6` = ?, \
         `subscribe_id` = ?, `geo_cn` = ?, \
         `rule_direct_ids` = ?, `rule_proxy_ids` = ?, `rule_reject_ids` = ?, \
         `include_area_non` = ?, `include_area` = ?, `include_name_contains` = ?, \
         `exclude_area` = ?, `exclude_name_contains` = ?, \
         `interval` = ?, `update_time` = ? where `id` = ?"
    );
    sqlx::query(&sql)
        .bind(&entity.name)
        .bind(entity.tun)
        .bind(entity.fake_ip)
        .bind(entity.ipv6)
        .bind(&entity.subscribe_id)
        .bind(entity.geo_cn)
        .bind(to_json(&entity.rule_direct_ids)?)
        .bind(to_json(&entity.rule_proxy_ids)?)
        .bind(to_json(&entity.rule_reject_ids)?)
        .bind(entity.include_area_non)
        .bind(to_json(&entity.include_area)?)
        .bind(to_json(&entity.include_name_contains)?)
        .bind(to_json(&entity.exclude_area)?)
        .bind(to_json(&entity.exclude_name_contains)?)
        .bind(entity.interval)
        .bind(entity.update_time)
        .bind(&entity.id)
        .execute(pool)
        .await?;
    Ok(())
}

/// 更新配置的刷新时间。
pub(crate) async fn update_refresh_time(pool: &DbPool, id: &str, millis: i64) -> Result<()> {
    let sql = format!("update {TABLE} set `refresh_time` = ? where `id` = ?");
    sqlx::query(&sql)
        .bind(millis)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// 按 id 删除配置。
pub(crate) async fn delete(pool: &DbPool, id: &str) -> Result<u64> {
    let sql = format!("delete from {TABLE} where `id` = ?");
    let result = sqlx::query(&sql).bind(id).execute(pool).await?;
    Ok(result.rows_affected())
}

fn to_json(values: &[String]) -> Result<String> {
    Ok(serde_json::to_string(values)?)
}

fn from_row(row: &DbRow) -> Result<Config> {
    Ok(Config {
        id: mapping::string(row, "id")?,
        name: mapping::string(row, "name")?,
        tun: mapping::boolean(row, "tun")?,
        fake_ip: mapping::boolean(row, "fake_ip")?,
        ipv6: mapping::boolean(row, "ipv6")?,
        subscribe_id: mapping::string(row, "subscribe_id")?,
        geo_cn: mapping::boolean(row, "geo_cn")?,
        rule_direct_ids: mapping::json_vec(row, "rule_direct_ids")?,
        rule_proxy_ids: mapping::json_vec(row, "rule_proxy_ids")?,
        rule_reject_ids: mapping::json_vec(row, "rule_reject_ids")?,
        include_area_non: mapping::boolean(row, "include_area_non")?,
        include_area: mapping::json_vec(row, "include_area")?,
        include_name_contains: mapping::json_vec(row, "include_name_contains")?,
        exclude_area: mapping::json_vec(row, "exclude_area")?,
        exclude_name_contains: mapping::json_vec(row, "exclude_name_contains")?,
        interval: mapping::millis(row, "interval")?,
        refresh_time: mapping::millis(row, "refresh_time")?,
        update_time: mapping::millis(row, "update_time")?,
        create_time: mapping::millis(row, "create_time")?,
    })
}
