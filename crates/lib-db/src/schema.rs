use crate::SqliteRow;
use sqlx::{QueryBuilder, Sqlite, SqlitePool, Transaction};

pub type DbPool = SqlitePool;
pub type DbRow = SqliteRow;
pub type DbQueryBuilder<'args> = QueryBuilder<'args, Sqlite>;
pub type DbTransaction<'c> = Transaction<'c, Sqlite>;
