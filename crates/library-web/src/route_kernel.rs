use crate::route_global::{IdPo, R};
use crate::singbox;
use crate::tbl_config::TblConfig;
use crate::tbl_setting::{TblSettingKernel, TblSettingRun};
use axum::routing::{get, post};
use axum::{Json, Router};
use library_core::app::get_app;
use library_core::app_config::AppConfig;
use library_core::core::AnyResult;
use library_core::file;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelState {
    pub running: bool,
    pub error: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_id: Option<String>,
    pub ui: String,
}

pub(crate) fn start(config: TblConfig) -> AnyResult<()> {
    TblSettingRun::set_selected(&config.id)?;
    let work_dir = get_app().cache_dir.join("sing_box");
    file::create_dir(&work_dir)?;
    let json = config.sing_box_json();
    singbox::start(&json, &work_dir)
}

fn _state() -> AnyResult<KernelState> {
    let _state = singbox::state()?;
    let map = AppConfig::keys(vec![TblSettingKernel::key_ui, TblSettingRun::key_selected])?;
    let ui = map
        .get(TblSettingKernel::key_ui)
        .map(|v| v.to_string())
        .unwrap_or_else(|| TblSettingKernel::default.ui.clone());
    let config_id = map.get(TblSettingRun::key_selected).map(|v| v.to_string());

    Ok(KernelState {
        running: _state.running,
        error: _state.error,
        reason: _state.reason,
        config_id,
        ui,
    })
}

async fn state() -> R<KernelState> {
    _state().into()
}

async fn _start(Json(po): Json<IdPo>) -> R<()> {
    let id = po.id.expect("必须指定启动配置!");
    let config = TblConfig::find(&id).unwrap().expect("未找到对应配置!");
    start(config).into()
}

async fn stop() -> R<()> {
    singbox::stop().into()
}

pub fn fill(router: Router) -> Router {
    router
        .route("/kernel/state", get(state))
        .route("/kernel/start", post(_start))
        .route("/kernel/stop", post(stop))
}
