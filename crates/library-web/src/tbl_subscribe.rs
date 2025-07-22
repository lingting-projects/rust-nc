use library_core::sqlite::StatementExt;
use serde::{Deserialize, Serialize};
use sqlite::Statement;
use std::convert::Into;

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
    /// 更新间隔, 单位: 毫秒
    pub interval: u32,
    /// 更新时间: 毫秒级别时间戳
    pub update_time: u128,
    /// 创建时间: 毫秒级别时间戳
    pub create_time: u128,
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
            download: stmt.read_u64("download").unwrap_or(0u64),
            upload: stmt.read_u64("upload").unwrap_or(0u64),
            max: stmt.read_u64("max").unwrap_or(0u64),
            expire_time: stmt.read_u128("expire_time").unwrap_or(0),
        }
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
