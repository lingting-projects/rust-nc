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

/// 发送 HEAD 请求。
pub(crate) async fn head(_request: FetchRequest) -> Result<FetchResponse> {
    bail!(MESSAGE)
}

/// 以 GET 请求取得响应体流。
pub(crate) async fn get_stream(_request: FetchRequest) -> Result<FetchStream> {
    bail!(MESSAGE)
}

/// 流式响应。
pub(crate) struct FetchStream;

impl FetchStream {
    pub(crate) fn is_success(&self) -> bool {
        false
    }

    pub(crate) fn status(&self) -> u16 {
        0
    }

    pub(crate) fn header(&self, _name: &str) -> Option<&String> {
        None
    }

    pub(crate) async fn next(&mut self) -> Option<Result<bytes::Bytes>> {
        None
    }
}
