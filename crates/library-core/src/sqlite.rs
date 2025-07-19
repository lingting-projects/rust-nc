mod v202507180;

use crate::app::APP;
use crate::app_config::AppConfig;
use crate::core::{AnyResult, BizError};
use sqlite::{open, Connection, ConnectionThreadSafe, Cursor, Row, State, Statement, Value};
use std::any::Any;
use std::sync::{Mutex, OnceLock};

static _CONN: OnceLock<ConnectionThreadSafe> = OnceLock::new();

fn _prepare<'conn>(sql: &str, args: Vec<Value>) -> AnyResult<Statement<'conn>> {
    let conn = _CONN.get().unwrap();
    let mut stmt = conn.prepare(sql)?;
    let mut i = 0;
    for arg in args {
        i += 1;
        stmt.bind((i, arg))?;
    }
    Ok(stmt)
}

pub fn query<E>(sql: &str, args: Vec<Value>, convert: fn(&Statement) -> E) -> AnyResult<Vec<E>> {
    let mut stmt = _prepare(sql, args)?;
    let mut vec = vec![];
    loop {
        match stmt.next() {
            Ok(n) => match n {
                State::Row => {
                    let e = convert(&stmt);
                    vec.push(e)
                }
                State::Done => break,
            },
            Err(e) => {
                log::error!("读取数据时异常! {}", e);
                break;
            }
        }
    }

    Ok(vec)
}

pub fn init() -> AnyResult<()> {
    if let Some(_) = _CONN.get() {
        return Ok(());
    }
    let path_db = APP.get().unwrap().data_dir.join("sqlite.db");
    log::debug!("数据库位置: {}", path_db.display());
    let safe = Connection::open_thread_safe(path_db)?;
    if let Err(_) = _CONN.set(safe) {
        Err(Box::new(BizError::SqliteInit))
    } else {
        let conn = _CONN.get().unwrap();
        log::debug!("初始化 app_config");
        AppConfig::init(conn)?;
        _version(conn)
    }
}

fn _version(conn: &ConnectionThreadSafe) -> AnyResult<()> {
    let version = AppConfig::version();
    log::debug!("当前版本: {}", version);
    if version < 20250718 {
        v202507180::init(conn)?
    }
    Ok(())
}
