use crate::core::AnyResult;
use crate::sqlite::{execute, query, StatementExt};
use sqlite::{ConnectionThreadSafe, Value};
use std::collections::HashMap;

// app_config
#[derive(Debug, Eq, PartialEq)]
pub struct AppConfig {
    // id
    pub key: String,
    pub value: String,
}

impl AppConfig {
    pub const table_name: &'static str = "app_config";

    pub const key_version: &'static str = "version";

    pub fn version() -> i32 {
        Self::get(Self::key_version)
            .ok()
            .flatten()
            .map(|s| s.parse::<i32>().ok())
            .flatten()
            .unwrap_or(-1)
    }

    pub fn version_set(v: i32) -> AnyResult<()> {
        Self::set(Self::key_version, &v.to_string())
    }

    pub fn find(key: &str) -> AnyResult<Option<Self>> {
        let vec = query(
            "select * from app_config where `key`=?",
            vec![key.into()],
            |r| {
                let key = r.read_string("key").expect("read key error");
                let value = r.read_string("value").expect("read value error");
                AppConfig { key, value }
            },
        )?;

        let v = vec.into_iter().next();
        Ok(v)
    }

    pub fn get(key: &str) -> AnyResult<Option<String>> {
        let vec = query(
            "select `value` from app_config where `key`=?",
            vec![key.into()],
            |r| {
                let value = r.read("value").expect("read value error");
                value
            },
        )?;

        let v = vec.into_iter().next();
        Ok(v)
    }

    pub fn get_or(key: &str, default: String) -> AnyResult<String> {
        let v = Self::get(key)?.unwrap_or(default);
        Ok(v)
    }

    pub fn set(key: &str, v: &str) -> AnyResult<()> {
        let sql = "replace into app_config(`key`,`value`) VALUES (?,?)";
        let args = vec![key.into(), v.into()];
        execute(sql, args).map(|_| ())
    }

    pub fn keys(keys: Vec<&str>) -> AnyResult<HashMap<String, String>> {
        let placeholders = keys.iter().map(|_| "?").collect::<Vec<_>>().join(",");

        let sql = format!("select * from app_config where `key` in ({})", placeholders);
        let args = keys.iter().map(|k| Value::from(*k)).collect();
        let vec = query(&sql, args, |r| {
            let key = r.read_string("key").expect("read key error");
            let value = r.read_string("value").expect("read value error");
            AppConfig { key, value }
        })?;

        let mut map = HashMap::with_capacity(keys.len());
        for v in vec {
            map.insert(v.key, v.value);
        }
        Ok(map)
    }

    pub(crate) fn init(conn: &ConnectionThreadSafe) -> AnyResult<()> {
        conn.execute(
            "
CREATE TABLE IF NOT EXISTS app_config
(
    key   TEXT PRIMARY KEY,
    value TEXT
);

INSERT OR IGNORE INTO app_config (key, value)
VALUES ('version', '-1');
        ",
        )?;

        Ok(())
    }
}
