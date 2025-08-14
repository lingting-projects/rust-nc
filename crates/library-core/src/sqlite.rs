mod v202507180;

use crate::app::get_app;
use crate::app_config::AppConfig;
use crate::boolean;
use crate::core::{AnyResult, BizError};
use sqlite::{
    open, ColumnIndex, Connection, ConnectionThreadSafe, Cursor, ParameterIndex, Row, State,
    Statement, Value,
};
use std::any::Any;
use std::sync::{Mutex, OnceLock};

// 扩展 trait 定义
pub trait StatementExt {
    fn read_string<U>(&self, col: U) -> Option<String>
    where
        U: ColumnIndex;
    fn read_i32<U>(&self, col: U) -> Option<i32>
    where
        U: ColumnIndex;
    fn read_u32<U>(&self, col: U) -> Option<u32>
    where
        U: ColumnIndex;
    fn read_u64<U>(&self, col: U) -> Option<u64>
    where
        U: ColumnIndex;
    fn read_u128<U>(&self, col: U) -> Option<u128>
    where
        U: ColumnIndex;
    fn read_bool<U>(&self, col: U) -> Option<bool>
    where
        U: ColumnIndex;
    #[cfg(feature = "json")]
    fn read_json_array<U>(&self, col: U) -> Option<Vec<serde_json::Value>>
    where
        U: ColumnIndex;
}

// 扩展 trait 实现
impl StatementExt for Statement<'_> {
    fn read_string<U>(&self, col: U) -> Option<String>
    where
        U: ColumnIndex,
    {
        match self.read::<Value, _>(col) {
            Ok(_v) => _v.string(),
            _ => None,
        }
    }

    fn read_i32<U>(&self, col: U) -> Option<i32>
    where
        U: ColumnIndex,
    {
        match self.read::<Value, _>(col) {
            Ok(Value::String(s)) => s.parse::<i32>().ok(),
            Ok(Value::Integer(i)) => Some(i as i32),
            Ok(Value::Float(f)) => Some(f as i32),
            _ => None,
        }
    }

    fn read_u32<U>(&self, col: U) -> Option<u32>
    where
        U: ColumnIndex,
    {
        match self.read::<Value, _>(col) {
            Ok(Value::String(s)) => s.parse::<u32>().ok(),
            Ok(Value::Integer(i)) => Some(i as u32),
            Ok(Value::Float(f)) => Some(f as u32),
            _ => None,
        }
    }

    fn read_u64<U>(&self, col: U) -> Option<u64>
    where
        U: ColumnIndex,
    {
        match self.read::<Value, _>(col) {
            Ok(Value::String(s)) => s.parse::<u64>().ok(),
            Ok(Value::Integer(i)) => Some(i as u64),
            Ok(Value::Float(f)) => Some(f as u64),
            _ => None,
        }
    }

    fn read_u128<U>(&self, col: U) -> Option<u128>
    where
        U: ColumnIndex,
    {
        match self.read::<Value, _>(col) {
            Ok(Value::String(s)) => s.parse::<u128>().ok(),
            Ok(Value::Integer(i)) => Some(i as u128),
            Ok(Value::Float(f)) => Some(f as u128),
            _ => None,
        }
    }

    fn read_bool<U>(&self, col: U) -> Option<bool>
    where
        U: ColumnIndex,
    {
        match self.read::<Value, _>(col) {
            Ok(Value::String(v)) => {
                if boolean::is_true(&v) {
                    Some(true)
                } else if boolean::is_false(&v) {
                    Some(false)
                } else {
                    None
                }
            }
            Ok(Value::Integer(i)) => Some(i > 0),
            Ok(Value::Float(f)) => Some(f > 0.0),
            _ => None,
        }
    }

    #[cfg(feature = "json")]
    fn read_json_array<U>(&self, col: U) -> Option<Vec<serde_json::Value>>
    where
        U: ColumnIndex,
    {
        match self.read::<Value, _>(col) {
            Ok(Value::String(v)) => {
                if v.trim().is_empty() || !v.starts_with("[") || !v.ends_with("]") {
                    None
                } else {
                    serde_json::from_str(&v).ok()
                }
            }
            _ => None,
        }
    }
}

pub trait SqliteValueExt {
    fn string(self) -> Option<String>;
}

impl SqliteValueExt for Value {
    fn string(self) -> Option<String> {
        match self {
            Value::Binary(v) => None,
            Value::Float(v) => Some(v.to_string()),
            Value::Integer(v) => Some(v.to_string()),
            Value::String(v) => Some(v),
            Value::Null => None,
        }
    }
}

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

pub fn execute(sql: &str, args: Vec<Value>) -> AnyResult<i32> {
    let mut stmt = _prepare(sql, args)?;

    match stmt.next() {
        Ok(n) => match n {
            State::Row => {
                let r = stmt.read_i32(0).expect("未读取到变更行数");
                Ok(r)
            }
            State::Done => Ok(0),
        },
        Err(e) => {
            log::error!("读取变更行数时异常! {}", e);
            Err(Box::new(e))
        }
    }
}

pub fn init() -> AnyResult<()> {
    if let Some(_) = _CONN.get() {
        return Ok(());
    }
    let path_db = get_app().data_dir.join("sqlite.db");
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
        log::debug!("更新到: 20250718");
        v202507180::init(conn)?
    }
    Ok(())
}
