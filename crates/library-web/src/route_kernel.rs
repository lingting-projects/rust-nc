use crate::route_global::{IdPo, R};
use crate::tbl_config::TblConfig;
use crate::tbl_setting::{TblSettingKernel, TblSettingRun};
use axum::routing::{get, post};
use axum::{Json, Router};
use library_core::app::APP;
use library_core::core::AnyResult;
use library_core::file;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelState {
    pub running: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_id: Option<String>,
    pub ui: String,
}

impl KernelState {
    pub fn new() -> AnyResult<Self> {
        Ok(Self {
            running: library_sing_box::is_running()?,
            config_id: TblSettingRun::selected()?,
            ui: TblSettingKernel::ui()?,
        })
    }
}

async fn state() -> R<KernelState> {
    KernelState::new().into()
}

async fn start(Json(po): Json<IdPo>) -> R<()> {
    let id = po.id.expect("必须指定启动配置!");
    let config = TblConfig::find(&id).unwrap().expect("未找到对应配置!");
    TblSettingRun::set_selected(&id).unwrap();
    let work_dir_path = APP.get().expect("failed get app").cache_dir.join("sing_box");
    file::create_dir(&work_dir_path).unwrap();
    library_sing_box::start(&config.sing_box_json(), &work_dir_path).into()
}

async fn stop() -> R<()> {
    library_sing_box::stop().into()
}

pub fn fill(router: Router) -> Router {
    router
        .route("/kernel/state", get(state))
        .route("/kernel/start", post(start))
        .route("/kernel/stop", post(stop))
}
