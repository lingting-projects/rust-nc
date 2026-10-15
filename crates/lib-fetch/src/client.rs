use anyhow::Result;
use bytes::Bytes;

use crate::request::{FetchRequest, HEADER_UA};
use crate::response::FetchResponse;

// wasm 固定使用 reqwest; 同时启用时优先使用 reqwest
#[cfg(all(
    not(any(feature = "wreq", feature = "reqwest")),
    not(target_arch = "wasm32")
))]
use crate::none_backend as backend;
#[cfg(any(feature = "reqwest", target_arch = "wasm32"))]
use crate::reqwest_backend as backend;
#[cfg(all(
    feature = "wreq",
    not(feature = "reqwest"),
    not(target_arch = "wasm32")
))]
use crate::wreq_backend as backend;

/// 请求使用的 User-Agent。
pub const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/135.0.0.0 Safari/537.36";

/// 远程资源请求客户端。
#[derive(Debug, Clone, Copy, Default)]
pub struct FetchClient;

impl FetchClient {
    /// 创建客户端。
    pub fn new() -> Self {
        Self
    }

    /// 发送 GET 请求。
    pub async fn get(&self, request: FetchRequest) -> Result<FetchResponse> {
        backend::get(with_ua(request)).await
    }

    /// 发送 POST 请求。
    pub async fn post(&self, request: FetchRequest) -> Result<FetchResponse> {
        backend::post(with_ua(request)).await
    }

    /// 以 GET 请求获取远程内容。
    pub async fn fetch(&self, url: &str) -> Result<FetchResponse> {
        self.get(FetchRequest::new(url)).await
    }

    /// 以 `body` 为请求体发送 POST 请求。
    pub async fn fetch_body(&self, url: &str, body: impl Into<Bytes>) -> Result<FetchResponse> {
        self.post(FetchRequest::new(url).with_body(body)).await
    }

    /// 以表单为请求体发送 POST 请求。
    pub async fn fetch_form(&self, url: &str, form: &[(&str, &str)]) -> Result<FetchResponse> {
        self.post(FetchRequest::new(url).with_form(form)).await
    }
}

/// 请求头中未设置 User-Agent 时补上内置的 User-Agent。
fn with_ua(mut request: FetchRequest) -> FetchRequest {
    if !request.has_ua() {
        request.headers.set(HEADER_UA, UA);
    }
    request
}
