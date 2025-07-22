use crate::route_global::R;
use crate::tbl_subscribe::{TblSubscribe, TblSubscribeUpsertDTO};
use axum::routing::{get, post};
use axum::{Json, Router};
use library_core::snowflake::next_str;
use library_core::sqlite::{execute, query};
use sqlite::Value;
use std::time::{SystemTime, UNIX_EPOCH};

async fn list() -> R<Vec<TblSubscribe>> {
    let sql = format!(
        "
select `id`,`name`,`url`, `interval`,`update_time`,`create_time`,`download`,`upload`,`max`,`expire_time`
from {}   ts ",
        TblSubscribe::table_name
    );
    query(&sql, vec![], |stmt| TblSubscribe::from_db(stmt)).into()
}

async fn upsert(Json(mut entity): Json<TblSubscribeUpsertDTO>) -> R<i32> {
    let sql: String;
    let args: Vec<Value>;

    let create = match entity.id.as_ref() {
        None => true,
        Some(v) => v.len() < 1,
    };
    let id: String = if create {
        next_str()
    } else {
        entity.id.unwrap()
    };
    let content = entity.content.unwrap_or("".into()).into();
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("get time err")
        .as_millis()
        .to_string();
    let time: Value = millis.to_string().into();
    let interval = entity.interval.to_string().into();

    if create {
        sql = format!(
            "
            insert into {}(`id`,`name`
            ,`url`,`content`,`nodes`
            ,`interval`,`update_time`,`create_time`
            ,`download`,`upload`,`max`,`expire_time`)
VALUES(?,?,?,?,?,?,?,?,?,?,?,?)
        ",
            TblSubscribe::table_name
        );
        args = vec![
            id.into(),
            entity.name.into(),
            entity.url.into(),
            content,
            "".into(),
            interval,
            time.clone(),
            time,
            0.into(),
            0.into(),
            0.into(),
            0.into(),
        ];
    } else {
        sql = format!(
            "update {} set `name`=?,`url`=?,`content`=?,`interval`=?,`update_time`=? where `id`=?",
            TblSubscribe::table_name
        );
        args = vec![
            entity.name.into(),
            entity.url.into(),
            content,
            interval,
            time,
            id.into(),
        ];
    }

    execute(&sql, args).into()
}

pub fn fill(router: Router) -> Router {
    router
        .route("/subscribe/list", get(list))
        .route("/subscribe/upsert", post(upsert))
}
