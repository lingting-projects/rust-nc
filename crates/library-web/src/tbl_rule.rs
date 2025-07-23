use crate::route_global::current_millis;
use library_core::app::APP;
use library_core::core::AnyResult;
use library_core::sqlite::{query, StatementExt};
use serde::{Deserialize, Serialize};
use sqlite::Statement;
use std::path::PathBuf;
use std::sync::LazyLock;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TblRule {
    pub id: String,
    pub name: String,
    /// 规则订阅地址, 空表示本地规则
    pub url: String,
    /// 订阅完整内容(url返回或者本地编辑的)
    pub content: String,
    /// 刷新间隔, 单位: 毫秒
    pub interval: u32,
    /// 更新时间: 毫秒级别时间戳
    pub update_time: u128,
    /// 创建时间: 毫秒级别时间戳
    pub create_time: u128,
    /// 刷新时间
    pub refresh_time: u128,
    /// 可用规则数量
    pub count: u64,
    /// 进程规则数量
    pub count_process: u64,
    /// IP规则数量
    pub count_ip: u64,
    /// 其他规则数量
    pub count_other: u64,
}

impl TblRule {
    pub const table_name: &'static str = "tbl_rule";

    pub const sql_field_content: &'static str =
        "CASE WHEN `url` LIKE 'http%' THEN '' ELSE `content` END AS content";

    pub fn from_db(stmt: &Statement) -> Self {
        Self {
            id: stmt.read_string("id").unwrap_or("".into()),
            name: stmt.read_string("name").unwrap_or("".into()),
            url: stmt.read_string("url").unwrap_or("".into()),
            content: stmt.read_string("content").unwrap_or("".into()),
            interval: stmt.read_u32("interval").unwrap_or(8640000),
            update_time: stmt.read_u128("update_time").unwrap_or(0),
            create_time: stmt.read_u128("create_time").unwrap_or(0),
            refresh_time: stmt.read_u128("refresh_time").unwrap_or(0),
            count: stmt.read_u64("count").unwrap_or(0),
            count_process: stmt.read_u64("count_process").unwrap_or(0),
            count_ip: stmt.read_u64("count_ip").unwrap_or(0),
            count_other: stmt.read_u64("count_other").unwrap_or(0),
        }
    }

    pub fn dir_data(id: String) -> PathBuf {
        APP.get().unwrap().data_dir.join("rule").join(id)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TblRuleUpsertDTO {
    pub id: Option<String>,
    pub name: String,
    pub url: String,
    pub content: Option<String>,
    pub interval: u32,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TblRuleRefreshDTO {
    pub id: String,
    pub name: String,
    pub url: String,
    pub content: String,
}

impl TblRuleRefreshDTO {
    pub fn from_db(stmt: &Statement) -> Self {
        Self {
            id: stmt.read_string("id").unwrap_or("".into()),
            name: stmt.read_string("name").unwrap_or("".into()),
            url: stmt.read_string("url").unwrap_or("".into()),
            content: stmt.read_string("content").unwrap_or("".into()),
        }
    }

    pub const sql_where_before: LazyLock<String> = LazyLock::new(|| {
        format!(
            "SELECT `id`,`name`,`url`,{} FROM {}",
            TblRule::sql_field_content,
            TblRule::table_name
        )
    });

    pub fn all() -> AnyResult<Vec<Self>> {
        let sql = Self::sql_where_before.clone();
        let args = vec![];

        query(&sql, args, |stmt| Self::from_db(stmt))
    }

    pub fn need_refresh() -> AnyResult<Vec<Self>> {
        let millis = current_millis();
        let sql = format!(
            "{} where `refresh_time`+`interval` <= cast(? as INTEGER)",
            Self::sql_where_before.clone(),
        );
        let args = vec![millis];

        query(&sql, args, |stmt| Self::from_db(stmt))
    }

    pub fn find(id: &str) -> AnyResult<Option<Self>> {
        let sql = format!("{} where `id` = ?", Self::sql_where_before.clone());
        let args = vec![id.into()];

        let vec = query(&sql, args, |stmt| Self::from_db(stmt))?;
        let option = vec.into_iter().find(|x| true);
        Ok(option)
    }

    pub fn dir_data(&self) -> PathBuf {
        TblRule::dir_data(self.id.clone())
    }
}
