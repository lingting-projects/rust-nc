use crate::route_global::R;
use crate::tbl_setting::TblSetting;
use axum::routing::{get, post};
use axum::{Json, Router};

async fn _get() -> R<TblSetting> {
    TblSetting::get().into()
}

async fn upsert(Json(entity): Json<TblSetting>) -> R<i32> {
    entity.upsert().into()
}

pub fn fill(router: Router) -> Router {
    router
        .route("/settings/get", get(_get))
        .route("/settings/upsert", post(upsert))
}
