use crate::core::AnyResult;
use crate::sqlite::query;
use sqlite::ConnectionThreadSafe;

// app_config
#[derive(Debug, Eq, PartialEq)]
pub struct AppConfig {
    // id
    pub key: String,
    pub value: String,
}

impl AppConfig {
    pub fn version() -> i32 {
        Self::get("version")
            .ok()
            .flatten()
            .map(|s| s.parse::<i32>().ok())
            .flatten()
            .unwrap_or(-1)
    }

    pub fn find(key: &str) -> AnyResult<Option<Self>> {
        let vec = query(
            "select * from app_config where `key`=?",
            vec![key.into()],
            |r| {
                let key = r.read("key").expect("read key error");
                let value = r.read("value").expect("read value error");
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
