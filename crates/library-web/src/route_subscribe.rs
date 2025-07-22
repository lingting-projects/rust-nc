use crate::http;
use crate::route_global::{current_millis, from_err_box, IdPo, R};
use crate::tbl_subscribe::{TblSubscribe, TblSubscribeRefreshDTO, TblSubscribeUpsertDTO};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use library_core::core::AnyResult;
use library_core::snowflake::next_str;
use library_core::sqlite::{execute, query};
use library_core::timer::Timer;
use library_nc::subscribe::{Subscribe, HEADER_INFO};
use sqlite::Value;
use std::convert::Into;
use std::ops::Deref;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

async fn _refresh_id(option: Option<String>) -> AnyResult<()> {
    if let Some(id) = option {
        let o = TblSubscribeRefreshDTO::find(&id)?;
        match o {
            None => Ok(()),
            Some(dto) => _refresh(dto).await,
        }
    } else {
        let vec = TblSubscribeRefreshDTO::all()?;
        for s in vec {
            _refresh(s).await?
        }
        Ok(())
    }
}

async fn _refresh(s: TblSubscribeRefreshDTO) -> AnyResult<()> {
    log::info!("[订阅] 刷新资源: {}", s.name);
    let content: Option<String>;
    let subscribe: Subscribe;
    if s.url.is_empty() {
        content = None;
        subscribe = Subscribe::resolve(&s.content, None)?;
    } else {
        let response = http::get(&s.url).await?;
        let info = response
            .headers()
            .get(HEADER_INFO)
            .map(|v| v.to_str().unwrap_or(""))
            .map(|o| o.to_string());
        let body = response.text().await?;
        subscribe = Subscribe::resolve(&body, info)?;
        content = Some(body);
    }
    if let Some(c) = content.clone() {
        if c == s.content {
            let time = current_millis();
            let sql = format!(
                "update {} set `refresh_time`=? where `id`=?",
                TblSubscribe::table_name
            );
            let args = vec![time, s.id.into()];
            execute(&sql, args)?;
            return Ok(());
        }
    }

    let json_nodes = serde_json::to_string(&subscribe.nodes)?;
    let time = current_millis();

    let sql = format!(
        "update {} set {}`nodes`=?,`refresh_time`=?,`download`=?,`upload`=?,`max`=?,`expire_time`=? where `id`=?",
        TblSubscribe::table_name,
        content.clone().map_or("", |v| "`content`=?,")
    );
    let mut args: Vec<Value> = vec![];
    if content.is_some() {
        args.push(content.unwrap().into());
    }
    args.push(json_nodes.into());
    args.push(time.into());
    args.push(subscribe.download.unwrap_or(0).to_string().into());
    args.push(subscribe.upload.unwrap_or(0).to_string().into());
    args.push(subscribe.max.unwrap_or(0).to_string().into());
    args.push(subscribe.expire.unwrap_or(0).to_string().into());
    args.push(s.id.into());

    execute(&sql, args)?;
    Ok(())
}

pub static TIMER_SUBSCRIBE: LazyLock<Arc<Timer>> = LazyLock::new(|| {
    Timer::new("Subscribe".into(), Duration::from_secs(60), || async {
        let vec = TblSubscribeRefreshDTO::need_refresh()?;
        for s in vec {
            let name = &s.name.clone();
            match _refresh(s).await {
                Ok(_) => {}
                Err(e) => {
                    log::error!("[订阅] [{}] 刷新异常! {}", name, e)
                }
            }
        }
        Ok(())
    })
});

async fn list() -> R<Vec<TblSubscribe>> {
    let sql = format!(
        "
select `id`,`name`,`url`, `interval`,`update_time`,`create_time`,`refresh_time`,`download`,`upload`,`max`,`expire_time`
from {}   ts ",
        TblSubscribe::table_name
    );
    query(&sql, vec![], |stmt| TblSubscribe::from_db(stmt)).into()
}

async fn upsert(Json(entity): Json<TblSubscribeUpsertDTO>) -> R<i32> {
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
    let time = current_millis();
    let interval = entity.interval.to_string().into();

    if create {
        sql = format!(
            "
            insert into {}(`id`,`name`
            ,`url`,`content`,`nodes`
            ,`interval`,`update_time`,`create_time`
            ,`refresh_time`,`download`,`upload`,`max`,`expire_time`)
VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?)
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

    let r = execute(&sql, args).into();
    TIMER_SUBSCRIBE.wake();
    r
}

async fn refresh(Json(po): Json<IdPo>) -> R<()> {
    _refresh_id(po.id).await.into()
}

pub fn fill(router: Router) -> Router {
    router
        .route("/subscribe/list", get(list))
        .route("/subscribe/upsert", post(upsert))
        .route("/subscribe/refresh", patch(refresh))
}
