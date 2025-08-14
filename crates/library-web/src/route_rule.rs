use crate::http::ResponseExt;
use crate::route_global::{current_millis, from_err_box, IdPo, R};
use crate::tbl_rule::{TblRule, TblRuleRefreshDTO, TblRuleUpsertDTO};
use crate::{http, singbox};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use library_core::app::APP;
use library_core::app_config::AppConfig;
use library_core::core::AnyResult;
use library_core::file;
use library_core::snowflake::next_str;
use library_core::sqlite::{execute, query};
use library_core::timer::Timer;
use library_nc::core::fast;
use library_nc::rule::{RuleType, SinBoxJsonRule};
use library_nc::subscribe::{Subscribe, HEADER_INFO};
use sqlite::Value;
use std::sync::{Arc, LazyLock};
use std::time::Duration;
use tokio::task::id;

async fn _refresh_id(option: Option<String>) -> AnyResult<()> {
    if let Some(id) = option {
        let o = TblRuleRefreshDTO::find(&id)?;
        match o {
            None => Ok(()),
            Some(dto) => _refresh(dto).await,
        }
    } else {
        let vec = TblRuleRefreshDTO::all()?;
        for s in vec {
            _refresh(s).await?
        }
        Ok(())
    }
}

async fn _refresh(s: TblRuleRefreshDTO) -> AnyResult<()> {
    log::info!("[规则] [{}] 刷新资源", s.name);
    let content: Option<String>;
    let sing_box: Vec<SinBoxJsonRule>;
    if s.url.is_empty() {
        log::info!("[规则] [{}] 本地规则", s.name);
        content = None;
        sing_box = SinBoxJsonRule::json_classical_process(s.content.as_str())?
    } else {
        log::info!("[规则] [{}] 远端规则", s.name);
        let url_fast = fast(&s.url);
        let response = http::get(&url_fast).await?;
        let body = response.read_text().await?;
        log::debug!("[规则] [{}] 获取到远端数据", s.name);
        sing_box = SinBoxJsonRule::json_classical_process(body.as_str())?;
        content = Some(body);
    }
    if let Some(c) = content.clone() {
        if c == s.content {
            log::info!("[规则] [{}] 规则内容未变更, 结束", s.name);
            let time = current_millis();
            let sql = format!(
                "update {} set `refresh_time`=? where `id`=?",
                TblRule::table_name
            );
            let args = vec![time, s.id.into()];
            execute(&sql, args)?;
            return Ok(());
        }
    }

    let root = s.dir_data();
    let mut count: u64 = 0;
    let mut count_process: u64 = 0;
    let mut count_ip: u64 = 0;
    let mut count_other: u64 = 0;

    log::debug!("[规则] [{}] 数据处理", s.name);
    for r in sing_box {
        count += r.count;

        match r.type_ {
            RuleType::Ip => count_ip += r.count,
            RuleType::Process => count_process += r.count,
            RuleType::Other => count_other += r.count,
        }

        let name = r.type_.name();
        log::debug!("[规则] [{}] SingBox 规则处理: {}", s.name, name);
        let path_json = root.join(format!("{}.json", name));
        let path_srs = root.join(format!("{}.srs", name));
        log::debug!("[规则] [{}] SingBox 清理旧数据文件: {}", s.name, name);
        file::delete(path_json.clone())?;
        file::delete(path_srs.clone())?;

        log::debug!("[规则] [{}] SingBox 写入json: {}", s.name, name);
        file::write(path_json.clone(), &r.json)?;
        log::debug!("[规则] [{}] SingBox 尝试转换为srs: {}", s.name, name);
        singbox::json_to_srs(&path_json, &path_srs)?;
    }

    log::debug!("[规则] [{}] 数据保存", s.name);
    let time = current_millis();
    let sql = format!(
        "update {} set {}`refresh_time`=?,`count`=?,`count_process`=?,`count_ip`=?,`count_other`=? where `id`=?",
        TblRule::table_name,
        content.clone().map_or("", |v| "`content`=?,")
    );
    let mut args: Vec<Value> = vec![];
    if content.is_some() {
        args.push(content.unwrap().into());
    }
    args.push(time.into());
    args.push(count.to_string().into());
    args.push(count_process.to_string().into());
    args.push(count_ip.to_string().into());
    args.push(count_other.to_string().into());
    args.push(s.id.into());

    execute(&sql, args)?;
    log::info!("[规则] [{}] 刷新完成", s.name);
    Ok(())
}

pub static TIMER_RULE: LazyLock<Arc<Timer>> = LazyLock::new(|| {
    Timer::new("Rule".into(), Duration::from_secs(60), || async {
        let vec = TblRuleRefreshDTO::need_refresh()?;
        for s in vec {
            let name = &s.name.clone();
            match _refresh(s).await {
                Ok(_) => {}
                Err(e) => {
                    log::error!("[规则] [{}] 刷新异常! {}", name, e)
                }
            }
        }
        Ok(())
    })
});

async fn list() -> R<Vec<TblRule>> {
    let sql = format!(
        "
select `id`,`name`,`url`, {}, `interval`,`update_time`,`create_time`,`refresh_time`,`count`,`count_process`,`count_ip`,`count_other`
from {}",
        TblRule::sql_field_content,
        TblRule::table_name
    );
    query(&sql, vec![], |stmt| TblRule::from_db(stmt)).into()
}

async fn upsert(Json(entity): Json<TblRuleUpsertDTO>) -> R<()> {
    let sql: String;
    let args: Vec<Value>;

    let create = match entity.id.as_ref() {
        None => true,
        Some(v) => v.is_empty(),
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
            ,`url`,`content`
            ,`interval`,`update_time`,`create_time`
            ,`refresh_time`,`count`,`count_process`,`count_ip`,`count_other`)
VALUES(?,?,?,?,?,?,?,?,?,?,?,?)
        ",
            TblRule::table_name
        );
        args = vec![
            id.clone().into(),
            entity.name.into(),
            entity.url.into(),
            content,
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
            TblRule::table_name
        );
        args = vec![
            entity.name.into(),
            entity.url.into(),
            content,
            interval,
            time,
            id.clone().into(),
        ];
    }
    _upsert(sql, args, id).await.into()
}

async fn _upsert(sql: String, args: Vec<Value>, id: String) -> AnyResult<()> {
    execute(&sql, args)?;
    _refresh_id(Some(id)).await
}

async fn refresh(Json(po): Json<IdPo>) -> R<()> {
    _refresh_id(po.id).await.into()
}

async fn delete(Json(po): Json<IdPo>) -> R<()> {
    if let Some(id) = po.id {
        let sql = format!("delete from {} where `id` = ? ", TblRule::table_name,);
        let args = vec![id.clone().into()];

        match execute(&sql, args) {
            Ok(_) => {
                let _ = file::delete_dir(TblRule::dir_data(&id));
            }
            Err(e) => return from_err_box(e),
        }
    }
    R::from(())
}

pub fn fill(router: Router) -> Router {
    router
        .route("/rule/list", get(list))
        .route("/rule/upsert", post(upsert))
        .route("/rule/refresh", patch(refresh))
        .route("/rule/delete", post(delete))
}
