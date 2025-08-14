use crate::http;
use crate::route_global::{current_millis, from_err_box, to_value, IdPo, R};
use crate::tbl_config::{TblConfig, TblConfigUpsertDTO};
use crate::tbl_rule::TblRule;
use crate::tbl_setting::TblSettingKernel;
use crate::tbl_subscribe::TblSubscribe;
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use library_core::app_config::AppConfig;
use library_core::core::{AnyResult, BizError};
use library_core::file;
use library_core::snowflake::next_str;
use library_core::sqlite::{execute, query};
use library_core::timer::Timer;
use library_nc::core::fast;
use library_nc::kernel::{
    dns_default_cn, dns_default_proxy, exclude_default, include_main, KernelConfig, NodeContains,
};
use library_nc::rule::{Rule, RuleType, SinBoxJsonRule};
use library_nc::subscribe::{Subscribe, SubscribeNode, HEADER_INFO};
use sqlite::Value;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock};
use std::time::Duration;
use tokio::task::id;

fn _rule_sing_box(
    config_root: &PathBuf,
    config: &TblConfig,
    ids: &Vec<String>,
) -> AnyResult<Vec<Rule>> {
    let mut vec = Vec::new();
    for id in ids {
        let root = TblRule::dir_data(id);
        log::debug!("[配置] [{}] [SingBox] 处理规则[{}]", config.name, id);

        for r in RuleType::all() {
            let mut path = root.join(format!("{}.srs", r.name()));
            if !path.exists() {
                path = root.join(format!("{}.json", r.name()));
            }
            if !path.exists() {
                log::debug!(
                    "[配置] [{}] [SingBox] 规则[{}]无可用[{}]类型配置",
                    config.name,
                    id,
                    r.name(),
                );
                continue;
            }

            let source = path
                .clone()
                .to_str()
                .ok_or_else(|| BizError::PathNotFound(path.clone()))?
                .to_string();

            log::debug!(
                "[配置] [{}] [SingBox] 规则[{}]找到[{}]类型配置: {}",
                config.name,
                id,
                r.name(),
                &source
            );

            let _filename = path
                .clone()
                .file_name()
                .ok_or_else(|| BizError::FileNotFound(source.clone()))?
                .to_str()
                .ok_or_else(|| BizError::FileNotFound(source.clone()))?
                .to_string();

            let filename = format!("rule_{}_{}", id, _filename);
            let rule_path = config_root.join(filename);
            let target = rule_path
                .clone()
                .to_str()
                .ok_or_else(|| BizError::PathNotFound(rule_path.clone()))?
                .to_string();

            log::debug!(
                "[配置] [{}] [SingBox] 规则[{}]的[{}]类型配置复制到: {}",
                config.name,
                id,
                r.name(),
                &target
            );
            file::copy_force(path, rule_path)?;

            let rule = Rule::from_local(*r, target);
            vec.push(rule);
        }
    }
    Ok(vec)
}

fn _build_sing_box(
    setting: &TblSettingKernel,
    config: &TblConfig,
    nodes: Vec<SubscribeNode>,
    include: &NodeContains,
    exclude: &NodeContains,
) -> AnyResult<()> {
    let root = config.sing_box_dir();
    log::debug!(
        "[配置] [{}] [SingBox] 获取直连规则数据: {}",
        config.name,
        config.rule_direct_ids.join(", ")
    );
    let rules_direct = _rule_sing_box(&root, &config, &config.rule_direct_ids)?;
    log::debug!(
        "[配置] [{}] [SingBox] 获取代理规则数据: {}",
        config.name,
        config.rule_proxy_ids.join(", ")
    );
    let rules_proxy = _rule_sing_box(&root, &config, &config.rule_proxy_ids)?;
    log::debug!(
        "[配置] [{}] [SingBox] 获取拒绝规则数据: {}",
        config.name,
        config.rule_reject_ids.join(", ")
    );
    let rules_reject = _rule_sing_box(&root, &config, &config.rule_reject_ids)?;

    let kc = KernelConfig {
        nodes,
        debug: false,
        tun: config.tun,
        fake_ip: config.fake_ip,
        ipv6: config.ipv6,
        geo_cn_direct: config.geo_cn,
        rules_direct,
        rules_proxy,
        rules_reject,
        dns_cn: setting.dns_cn.clone(),
        dns_proxy: setting.dns_proxy.clone(),
    }
    .with_default(include, exclude);

    if kc.nodes.is_empty() {
        return Err(Box::new(BizError::NodesEmpty(config.id.clone())));
    }

    log::debug!("[配置] [{}] [SingBox] 构建配置", config.name,);
    let json = kc.sing_box(&setting.ui, &setting.mixed_listen, setting.mixed_port)?;
    log::debug!("[配置] [{}] [SingBox] 写入配置", config.name,);
    let path = config.sing_box_json();
    file::overwrite(path, &json)?;
    Ok(())
}

async fn _refresh_id(option: Option<String>) -> AnyResult<()> {
    if let Some(id) = option {
        let o = TblConfig::find(&id)?;
        match o {
            None => Ok(()),
            Some(dto) => _refresh(dto).await,
        }
    } else {
        let vec = TblConfig::all()?;
        for s in vec {
            _refresh(s).await?
        }
        Ok(())
    }
}

async fn _refresh(config: TblConfig) -> AnyResult<()> {
    log::info!("[配置] [{}] 刷新配置", config.name);
    log::debug!(
        "[配置] [{}] 获取订阅数据: {}",
        config.name,
        config.subscribe_id
    );
    let nodes = TblSubscribe::find_nodes(&config.subscribe_id)?;
    if nodes.is_empty() {
        return Err(Box::new(BizError::NodesEmpty(config.id.clone())));
    }
    log::info!("[配置] [{}] 获取内核设置", config.name);
    let setting = TblSettingKernel::get()?;
    log::info!("[配置] [{}] 构建包含规则", config.name);
    let include = NodeContains {
        non_area: config.include_area_non,
        non_name: include_main.non_name,
        area: config.include_area.clone(),
        name_contains: config.include_name_contains.clone(),
    };
    log::info!("[配置] [{}] 构建排除规则", config.name);
    let exclude = NodeContains {
        area: config.exclude_area.clone(),
        name_contains: config.exclude_name_contains.clone(),
        non_area: exclude_default.non_area,
        non_name: exclude_default.non_name,
    };
    log::debug!(
        "[配置] [{}] 刷新SingBox配置: {}",
        config.name,
        config.subscribe_id
    );
    _build_sing_box(&setting, &config, nodes, &include, &exclude)?;
    log::info!("[配置] [{}] 刷新完成", config.name);

    let time = current_millis();
    let sql = format!(
        "update {} set `refresh_time`=? where `id`=?",
        TblConfig::table_name
    );
    let args = vec![time, config.id.into()];
    execute(&sql, args)?;
    Ok(())
}

pub static TIMER_CONFIG: LazyLock<Arc<Timer>> = LazyLock::new(|| {
    Timer::new("Rule".into(), Duration::from_secs(60), || async {
        let vec = TblConfig::need_refresh()?;
        for s in vec {
            let name = &s.name.clone();
            match _refresh(s).await {
                Ok(_) => {}
                Err(e) => {
                    log::error!("[配置] [{}] 刷新异常! {}", name, e)
                }
            }
        }
        Ok(())
    })
});

async fn list() -> R<Vec<TblConfig>> {
    TblConfig::all().into()
}

async fn upsert(Json(entity): Json<TblConfigUpsertDTO>) -> R<()> {
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
    let time = current_millis();
    let interval = entity.interval.to_string().into();
    let tun = to_value(entity.tun);
    let fake_ip = to_value(entity.fake_ip);
    let ipv6 = to_value(entity.ipv6);
    let geo_cn = to_value(entity.geo_cn);
    let rule_direct_ids = serde_json::to_string(&serde_json::Value::Array(
        entity
            .rule_direct_ids
            .into_iter()
            .map(|s| serde_json::Value::String(s))
            .collect(),
    ))
    .unwrap()
    .into();
    let rule_proxy_ids = serde_json::to_string(&serde_json::Value::Array(
        entity
            .rule_proxy_ids
            .into_iter()
            .map(|s| serde_json::Value::String(s))
            .collect(),
    ))
    .unwrap()
    .into();
    let rule_reject_ids = serde_json::to_string(&serde_json::Value::Array(
        entity
            .rule_reject_ids
            .into_iter()
            .map(|s| serde_json::Value::String(s))
            .collect(),
    ))
    .unwrap()
    .into();
    let include_area_non = to_value(entity.include_area_non);
    let include_area = serde_json::to_string(&serde_json::Value::Array(
        entity
            .include_area
            .into_iter()
            .map(|s| serde_json::Value::String(s))
            .collect(),
    ))
    .unwrap()
    .into();
    let include_name_contains = serde_json::to_string(&serde_json::Value::Array(
        entity
            .include_name_contains
            .into_iter()
            .map(|s| serde_json::Value::String(s))
            .collect(),
    ))
    .unwrap()
    .into();
    let exclude_area = serde_json::to_string(&serde_json::Value::Array(
        entity
            .exclude_area
            .into_iter()
            .map(|s| serde_json::Value::String(s))
            .collect(),
    ))
    .unwrap()
    .into();
    let exclude_name_contains = serde_json::to_string(&serde_json::Value::Array(
        entity
            .exclude_name_contains
            .into_iter()
            .map(|s| serde_json::Value::String(s))
            .collect(),
    ))
    .unwrap()
    .into();

    if create {
        sql = format!(
            "
            insert into {}(`id`,`name`
            ,`tun`,`fake_ip`,`ipv6`
            ,`subscribe_id`,`geo_cn`
            ,`rule_direct_ids`,`rule_proxy_ids`,`rule_reject_ids`
            ,`include_area_non`,`include_area`,`include_name_contains`
            ,`exclude_area`,`exclude_name_contains`
            ,`interval`,`refresh_time`,`update_time`,`create_time`)
VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
        ",
            TblConfig::table_name
        );
        args = vec![
            id.clone().into(),
            entity.name.into(),
            tun,
            fake_ip,
            ipv6,
            entity.subscribe_id.into(),
            geo_cn,
            rule_direct_ids,
            rule_proxy_ids,
            rule_reject_ids,
            include_area_non,
            include_area,
            include_name_contains,
            exclude_area,
            exclude_name_contains,
            interval,
            0.into(),
            time.clone(),
            time,
        ];
    } else {
        sql = format!(
            "update {} set `name`=?
            ,`tun`=?,`fake_ip`=?,`ipv6`=?
            ,`subscribe_id`=?,`geo_cn`=?
            ,`rule_direct_ids`=?,`rule_proxy_ids`=?,`rule_reject_ids`=?
            ,`include_area_non`=?,`include_area`=?,`include_name_contains`=?
            ,`exclude_area`=?,`exclude_name_contains`=?
            ,`interval`=?,`update_time`=? where `id`=?",
            TblConfig::table_name
        );
        args = vec![
            entity.name.into(),
            tun,
            fake_ip,
            ipv6,
            entity.subscribe_id.into(),
            geo_cn,
            rule_direct_ids,
            rule_proxy_ids,
            rule_reject_ids,
            include_area_non,
            include_area,
            include_name_contains,
            exclude_area,
            exclude_name_contains,
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
        let sql = format!("delete from {} where `id` = ? ", TblConfig::table_name,);
        let args = vec![id.clone().into()];

        match execute(&sql, args) {
            Ok(_) => {
                let _ = file::delete_dir(TblConfig::dir_data(&id));
            }
            Err(e) => return from_err_box(e),
        }
    }
    R::from(())
}

async fn default() -> R<TblConfigUpsertDTO> {
    R::from(TblConfigUpsertDTO::default.clone())
}

pub fn fill(router: Router) -> Router {
    router
        .route("/config/list", get(list))
        .route("/config/upsert", post(upsert))
        .route("/config/refresh", patch(refresh))
        .route("/config/delete", post(delete))
        .route("/config/default", get(default))
}
