//! 远程请求与下载。
//!
//! 对外提供两件事：一次性的请求响应（[`FetchClient`] + [`FetchRequest`] / [`FetchResponse`]），
//! 以及带探测的并发下载（[`FetchClient::probe`] 判断是否支持分段，[`FetchClient::download`]
//! 据此决定分片并行还是单点下载）。
//! 后端在 `wreq`、`reqwest`、无后端三者中按特性择一，调用方不感知差异。

mod client;
#[cfg(not(target_arch = "wasm32"))]
mod download;
#[cfg(all(
    not(any(feature = "wreq", feature = "reqwest")),
    not(target_arch = "wasm32")
))]
mod none_backend;
mod request;
#[cfg(any(feature = "reqwest", target_arch = "wasm32"))]
mod reqwest_backend;
mod response;
#[cfg(any(feature = "wreq", feature = "reqwest", target_arch = "wasm32"))]
mod util;
#[cfg(all(
    feature = "wreq",
    not(feature = "reqwest"),
    not(target_arch = "wasm32")
))]
mod wreq_backend;

// 后端实现三选一, 对外统一以 `backend` 引用
#[cfg(all(
    not(any(feature = "wreq", feature = "reqwest")),
    not(target_arch = "wasm32")
))]
use none_backend as backend;
#[cfg(any(feature = "reqwest", target_arch = "wasm32"))]
use reqwest_backend as backend;
#[cfg(all(
    feature = "wreq",
    not(feature = "reqwest"),
    not(target_arch = "wasm32")
))]
use wreq_backend as backend;

pub use client::{FetchClient, UA};
#[cfg(not(target_arch = "wasm32"))]
pub use download::{DEFAULT_THREADS, DownloadOutcome, RemoteProbe};
pub use request::{FetchRequest, HEADER_UA};
pub use response::FetchResponse;
