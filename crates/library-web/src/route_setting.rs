use crate::route_global::{from_err_box, R};
use crate::tbl_setting::TblSetting;
use crate::updater;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::{LazyLock, Mutex};

async fn _get() -> R<TblSetting> {
    TblSetting::get().into()
}

async fn upsert(Json(entity): Json<TblSetting>) -> R<i32> {
    entity.upsert().into()
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateState {
    pub checking: bool,
    pub new: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_size: Option<String>,
    pub downloading: bool,
    pub unzipping: bool,
    pub installing: bool,
    pub installed: bool,
    pub error: bool,
    /// 异常文本
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl UpdateState {
    fn running(&self) -> bool {
        self.checking || self.downloading || self.unzipping || self.installed
    }
}

static STATE: LazyLock<Mutex<UpdateState>> = LazyLock::new(|| Mutex::new(UpdateState::default()));

async fn state() -> R<UpdateState> {
    match STATE.lock() {
        Ok(s) => s.clone().into(),
        Err(e) => from_err_box(Box::new(e)),
    }
}

async fn update_check() -> R<bool> {
    let mut guard = STATE.lock().expect("failed get state");
    if guard.running() {
        return true.into();
    }
    match updater::check() {
        Ok(Some((version, url, size))) => {
            let mut state = UpdateState::default();
            state.new = true;
            state.new_version = Some(version);
            state.new_url = Some(url);
            state.new_size = Some(size.display());

            *guard = state;
            true.into()
        }
        Ok(None) => false.into(),
        Err(e) => from_err_box(e),
    }
}

fn on_download() {
    let mut guard = STATE.lock().expect("failed get state");
    let state = &mut *guard;
    state.checking = false;
    state.downloading = true;
}

fn on_install() {
    let mut guard = STATE.lock().expect("failed get state");
    let state = &mut *guard;
    state.checking = false;
    state.downloading = false;
    state.installing = true;
}

async fn update_install() -> R<bool> {
    let guard = STATE.lock().expect("failed get state");
    if guard.running() && !guard.checking {
        return false.into();
    }

    let listener = updater::UpdateListener {
        url: guard.new_url.clone().expect("获取下载地址异常!"),
        on_download: Box::new(on_download),
        on_install: Box::new(on_install),
    };

    match updater::update(listener) {
        Ok(_) => true.into(),
        Err(e) => from_err_box(e),
    }
}

pub fn fill(router: Router) -> Router {
    router
        .route("/settings/get", get(_get))
        .route("/settings/upsert", post(upsert))
        .route("/settings/update/state", get(state))
        .route("/settings/update/check", post(update_check))
        .route("/settings/update/install", post(update_install))
}
