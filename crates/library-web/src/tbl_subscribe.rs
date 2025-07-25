use crate::route_global::current_millis;
use library_core::core::AnyResult;
use library_core::sqlite::{query, StatementExt};
use library_nc::subscribe::SubscribeNode;
use serde::{Deserialize, Serialize};
use sqlite::Statement;
use std::convert::Into;
use std::sync::LazyLock;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TblSubscribe {
    pub id: String,
    pub name: String,
    /// 订阅地址
    pub url: String,
    /// 订阅地址返回完整内容
    pub content: String,
    /// 订阅解析后的所有节点
    pub nodes: String,
    /// 刷新间隔, 单位: 毫秒
    pub interval: u32,
    /// 更新时间: 毫秒级别时间戳
    pub update_time: u128,
    /// 创建时间: 毫秒级别时间戳
    pub create_time: u128,
    /// 刷新时间
    pub refresh_time: u128,
    /// 下载流量, 单位: Bytes
    pub download: u64,
    /// 上传流量, 单位: Bytes
    pub upload: u64,
    /// 最大可用流量, 单位: Bytes
    pub max: u64,
    /// 过期时间: 毫秒级别时间戳
    pub expire_time: u128,
}

impl TblSubscribe {
    pub const table_name: &'static str = "tbl_subscribe";

    pub const sql_field_content: &'static str =
        "CASE WHEN `url` LIKE 'http%' THEN '' ELSE `content` END AS content";

    pub fn from_db(stmt: &Statement) -> Self {
        Self {
            id: stmt.read_string("id").unwrap_or("".into()),
            name: stmt.read_string("name").unwrap_or("".into()),
            url: stmt.read_string("url").unwrap_or("".into()),
            content: stmt.read_string("content").unwrap_or("".into()),
            nodes: stmt.read_string("nodes").unwrap_or("".into()),
            interval: stmt.read_u32("interval").unwrap_or(8640000),
            update_time: stmt.read_u128("update_time").unwrap_or(0),
            create_time: stmt.read_u128("create_time").unwrap_or(0),
            refresh_time: stmt.read_u128("refresh_time").unwrap_or(0),
            download: stmt.read_u64("download").unwrap_or(0),
            upload: stmt.read_u64("upload").unwrap_or(0),
            max: stmt.read_u64("max").unwrap_or(0),
            expire_time: stmt.read_u128("expire_time").unwrap_or(0),
        }
    }

    pub fn find_nodes(id: &String) -> AnyResult<Vec<SubscribeNode>> {
        let sql = format!(
            "select `nodes` from {} where `id` = ? limit 1",
            Self::table_name
        );
        let args = vec![id.to_string().into()];

        let _vec = query(&sql, args, |s| s.read_string("nodes"))?;
        for x in _vec {
            if let Some(json) = x {
                let vec = serde_json::from_str::<Vec<SubscribeNode>>(&json)?;

                if !vec.is_empty() {
                    return Ok(vec);
                }
            }
        }

        Ok(vec![])
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TblSubscribeUpsertDTO {
    pub id: Option<String>,
    pub name: String,
    pub url: String,
    pub content: Option<String>,
    pub interval: u32,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TblSubscribeRefreshDTO {
    pub id: String,
    pub name: String,
    pub url: String,
    pub content: String,
}

impl TblSubscribeRefreshDTO {
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
            TblSubscribe::sql_field_content,
            TblSubscribe::table_name
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
}
