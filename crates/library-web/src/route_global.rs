use axum::response::Response;
use axum::{response::IntoResponse, Json, Router};
use library_core::core::AnyResult;
use serde::Serialize;
use tower_http::cors::{AllowHeaders, AllowMethods, Any, CorsLayer, ExposeHeaders};

pub fn fill(router: Router) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(AllowMethods::any())
        .allow_headers(AllowHeaders::any())
        .expose_headers(ExposeHeaders::any());
    router.route_layer(cors)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct R<T> {
    code: i32,
    message: String,
    data: Option<T>,
}

pub static ok_code: i32 = 200;
pub static ok_msg: &str = "success";

impl<T> From<Result<T, String>> for R<T> {
    fn from(result: Result<T, String>) -> Self {
        match result {
            Ok(data) => Self {
                code: ok_code,
                message: ok_msg.to_string(),
                data: Some(data),
            },
            Err(err) => Self {
                code: 1,
                message: err,
                data: None,
            },
        }
    }
}

impl<T> From<AnyResult<T>> for R<T> {
    fn from(result: AnyResult<T>) -> Self {
        match result {
            Ok(data) => Self {
                code: ok_code,
                message: ok_msg.to_string(),
                data: Some(data),
            },
            Err(err) => Self {
                code: 1,
                message: err.to_string(),
                data: None,
            },
        }
    }
}

impl<T> From<T> for R<T> {
    fn from(value: T) -> Self {
        Self {
            code: ok_code,
            message: ok_msg.to_string(),
            data: Some(value),
        }
    }
}

impl<T> IntoResponse for R<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}
