use crate::route_global::current_millis;
use library_core::app::get_app;
use library_core::core::AnyResult;
use library_core::json::JsonValueExt;
use library_core::sqlite::{query, StatementExt};
use library_nc::kernel::{exclude_default, include_main};
use serde::{Deserialize, Serialize};
use sqlite::Statement;
use std::clone::Clone;
use std::convert::Into;
use std::path::PathBuf;
use std::sync::LazyLock;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TblConfig {
    pub id: String,
    pub name: String,
    pub tun: bool,
    pub fake_ip: bool,
    pub ipv6: bool,
    /// 订阅
    pub subscribe_id: String,
    /// geo cn 直连
    pub geo_cn: bool,
    /// 规则 id1,id2
    pub rule_direct_ids: Vec<String>,
    pub rule_proxy_ids: Vec<String>,
    pub rule_reject_ids: Vec<String>,
    /// 包含
    pub include_area_non: bool,
    /// 包含指定区域 json字符串
    pub include_area: Vec<String>,
    /// 包含名称中存在关键的 json字符串
    pub include_name_contains: Vec<String>,
    /// 排除
    /// 排除指定区域 json字符串
    pub exclude_area: Vec<String>,
    /// 排除名称中存在关键的 json字符串
    pub exclude_name_contains: Vec<String>,
    /// 刷新间隔, 单位: 毫秒
    pub interval: u32,
    /// 刷新时间
    pub refresh_time: u128,
    /// 更新时间: 毫秒级别时间戳
    pub update_time: u128,
    /// 创建时间: 毫秒级别时间戳
    pub create_time: u128,
}

impl TblConfig {
    pub const table_name: &'static str = "tbl_config";

    pub fn from_db(stmt: &Statement) -> Self {
        Self {
            id: stmt.read_string("id").unwrap_or("".into()),
            name: stmt
                .read_string("name")
                .unwrap_or(TblConfigUpsertDTO::default.name.clone()),
            tun: stmt
                .read_bool("tun")
                .unwrap_or(TblConfigUpsertDTO::default.tun.clone()),
            fake_ip: stmt
                .read_bool("fake_ip")
                .unwrap_or(TblConfigUpsertDTO::default.fake_ip.clone()),
            ipv6: stmt
                .read_bool("ipv6")
                .unwrap_or(TblConfigUpsertDTO::default.ipv6.clone()),
            subscribe_id: stmt
                .read_string("subscribe_id")
                .unwrap_or(TblConfigUpsertDTO::default.subscribe_id.clone()),
            geo_cn: stmt
                .read_bool("geo_cn")
                .unwrap_or(TblConfigUpsertDTO::default.geo_cn.clone()),
            rule_direct_ids: stmt
                .read_json_array("rule_direct_ids")
                .map(|v| v.into_iter().map(|_v| _v.string()).flatten().collect())
                .unwrap_or(TblConfigUpsertDTO::default.rule_direct_ids.clone()),
            rule_proxy_ids: stmt
                .read_json_array("rule_proxy_ids")
                .map(|v| v.into_iter().map(|_v| _v.string()).flatten().collect())
                .unwrap_or(TblConfigUpsertDTO::default.rule_proxy_ids.clone()),
            rule_reject_ids: stmt
                .read_json_array("rule_reject_ids")
                .map(|v| v.into_iter().map(|_v| _v.string()).flatten().collect())
                .unwrap_or(TblConfigUpsertDTO::default.rule_reject_ids.clone()),
            include_area_non: stmt
                .read_bool("include_area_non")
                .unwrap_or(TblConfigUpsertDTO::default.include_area_non.clone()),
            include_area: stmt
                .read_json_array("include_area")
                .map(|v| v.into_iter().map(|_v| _v.string()).flatten().collect())
                .unwrap_or(TblConfigUpsertDTO::default.include_area.clone()),
            include_name_contains: stmt
                .read_json_array("include_name_contains")
                .map(|v| v.into_iter().map(|_v| _v.string()).flatten().collect())
                .unwrap_or(TblConfigUpsertDTO::default.include_name_contains.clone()),
            exclude_area: stmt
                .read_json_array("exclude_area")
                .map(|v| v.into_iter().map(|_v| _v.string()).flatten().collect())
                .unwrap_or(TblConfigUpsertDTO::default.exclude_area.clone()),
            exclude_name_contains: stmt
                .read_json_array("exclude_name_contains")
                .map(|v| v.into_iter().map(|_v| _v.string()).flatten().collect())
                .unwrap_or(TblConfigUpsertDTO::default.exclude_name_contains.clone()),
            interval: stmt
                .read_u32("interval")
                .unwrap_or(TblConfigUpsertDTO::default.interval.clone()),
            refresh_time: stmt.read_u128("refresh_time").unwrap_or(0),
            update_time: stmt.read_u128("update_time").unwrap_or(0),
            create_time: stmt.read_u128("create_time").unwrap_or(0),
        }
    }

    pub fn find(id: &str) -> AnyResult<Option<TblConfig>> {
        let sql = format!("select * from {} where `id`=?", TblConfig::table_name);
        let vec = query(&sql, vec![id.into()], |stmt| TblConfig::from_db(stmt))?;
        Ok(vec.into_iter().find(|_| true))
    }

    pub fn all() -> AnyResult<Vec<TblConfig>> {
        let sql = format!("select * from {}", TblConfig::table_name);
        query(&sql, vec![], |stmt| TblConfig::from_db(stmt))
    }

    pub fn need_refresh() -> AnyResult<Vec<Self>> {
        let millis = current_millis();
        let sql = format!(
            "select * from {} where `refresh_time`+`interval` <= cast(? as INTEGER)",
            Self::table_name,
        );
        let args = vec![millis];

        query(&sql, args, |stmt| Self::from_db(stmt))
    }

    pub fn dir_data(id: &str) -> PathBuf {
        get_app().data_dir.join("config").join(id)
    }

    pub fn sing_box_dir(&self) -> PathBuf {
        Self::dir_data(&self.id).join("sing_box")
    }

    pub fn sing_box_json(&self) -> PathBuf {
        self.sing_box_dir().join("config.json")
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TblConfigUpsertDTO {
    pub id: Option<String>,
    pub name: String,
    pub tun: bool,
    pub fake_ip: bool,
    pub ipv6: bool,
    /// 订阅
    pub subscribe_id: String,
    /// geo cn 直连
    pub geo_cn: bool,
    /// 规则 id1,id2
    pub rule_direct_ids: Vec<String>,
    pub rule_proxy_ids: Vec<String>,
    pub rule_reject_ids: Vec<String>,
    /// 包含
    pub include_area_non: bool,
    /// 包含指定区域 json字符串
    pub include_area: Vec<String>,
    /// 包含名称中存在关键的 json字符串
    pub include_name_contains: Vec<String>,
    /// 排除
    /// 排除指定区域 json字符串
    pub exclude_area: Vec<String>,
    /// 排除名称中存在关键的 json字符串
    pub exclude_name_contains: Vec<String>,
    /// 刷新间隔, 单位: 毫秒
    pub interval: u32,
}

impl TblConfigUpsertDTO {
    pub const default: LazyLock<TblConfigUpsertDTO> = LazyLock::new(|| TblConfigUpsertDTO {
        id: None,
        name: "".to_string(),
        tun: true,
        fake_ip: true,
        ipv6: true,
        subscribe_id: "".to_string(),
        geo_cn: true,
        rule_direct_ids: vec![],
        rule_proxy_ids: vec![],
        rule_reject_ids: vec![],
        include_area_non: true,
        include_area: include_main.area.clone(),
        include_name_contains: include_main.name_contains.clone(),
        exclude_area: exclude_default.area.clone(),
        exclude_name_contains: exclude_default.name_contains.clone(),
        interval: 36000000,
    });
}
