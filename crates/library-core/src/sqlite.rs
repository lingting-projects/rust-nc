use crate::app::APP;
use crate::core::{AnyResult, BizError};
use sqlite::{open, Connection};
use std::sync::{Mutex, OnceLock};

static _CONN: OnceLock<Mutex<Connection>> = OnceLock::new();

pub fn init() -> AnyResult<()> {
    if let Some(_) = _CONN.get() {
        return Ok(());
    }
    let path_db = APP.get().unwrap().data_dir.join("sqlite.db");
    log::debug!("数据库位置: {}", path_db.display());
    let connection = open(path_db)?;
    if let Err(_) = _CONN.set(Mutex::new(connection)) {
        Err(Box::new(BizError::SqliteInit))
    } else {
        Ok(())
    }
}
