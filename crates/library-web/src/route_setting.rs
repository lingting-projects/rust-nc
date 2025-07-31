use crate::route_global::R;
use crate::tbl_setting::{TblAppState, TblSetting};
use axum::routing::{get, post};
use axum::{Json, Router};

async fn _get() -> R<TblSetting> {
    TblSetting::get().into()
}

async fn upsert(Json(entity): Json<TblSetting>) -> R<i32> {
    entity.upsert().into()
}

async fn state() -> R<TblAppState> {
    TblAppState::new().into()
}

async fn update_check() -> R<bool> {
    R::from(false)
}

async fn update_install() -> R<bool> {
    R::from(false)
}

pub fn fill(router: Router) -> Router {
    router
        .route("/settings/get", get(_get))
        .route("/settings/upsert", post(upsert))
        .route("/settings/state", get(state))
        .route("/settings/update/check", post(update_check))
        .route("/settings/update/install", post(update_install))
}
