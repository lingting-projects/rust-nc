//! 未启用任何后端时的占位实现, 请求在运行时返回错误。

use anyhow::{Result, bail};

use crate::request::FetchRequest;
use crate::response::FetchResponse;

/// 未启用后端时的错误信息。
const MESSAGE: &str = "lib-fetch 未启用任何后端, 请启用 `wreq` 或 `reqwest` 特性";

/// 发送 GET 请求。
pub(crate) async fn get(_request: FetchRequest) -> Result<FetchResponse> {
    bail!(MESSAGE)
}

/// 发送 POST 请求。
pub(crate) async fn post(_request: FetchRequest) -> Result<FetchResponse> {
    bail!(MESSAGE)
}
