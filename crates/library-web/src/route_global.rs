use axum::response::Response;
use axum::{response::IntoResponse, Json, Router};
use library_core::core::AnyResult;
use log::log;
use serde::{Deserialize, Serialize};
use serde_json::map::Entry::Vacant;
use sqlite::Value;
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};
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
    pub code: i32,
    pub message: String,
    pub data: Option<T>,
}

pub static ok_code: i32 = 200;
pub static ok_msg: &str = "success";

pub fn from_err_box<T>(b: Box<dyn Error>) -> R<T> {
    log::error!("请求处理异常! {}", b);
    R {
        code: 500,
        message: b.to_string(),
        data: None,
    }
}

impl<T> From<Result<T, String>> for R<T> {
    fn from(result: Result<T, String>) -> Self {
        match result {
            Ok(data) => Self {
                code: ok_code,
                message: ok_msg.to_string(),
                data: Some(data),
            },
            Err(err) => {
                log::error!("请求处理异常! {}", err);
                Self {
                    code: 500,
                    message: err,
                    data: None,
                }
            }
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
            Err(err) => {
                log::error!("请求处理异常! {}", err);
                Self {
                    code: 500,
                    message: err.to_string(),
                    data: None,
                }
            }
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

pub fn current_millis() -> Value {
    let millis = library_core::core::current_millis().expect("get time err");
    let time: Value = millis.to_string().into();
    time
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdPo {
    pub id: Option<String>,
}

// region value 扩展

pub fn to_value(f: bool) -> Value {
    if f { Value::from(1) } else { Value::from(0) }
}

// endregion
