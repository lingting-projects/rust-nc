//! 远程资源获取。
//!
//! 通过 `wreq`(模拟 chrome 的 tls 指纹) 或 `reqwest` 提供统一的请求接口。
//! 两者同时启用时使用 `reqwest`; 均未启用时不报编译错误, 请求在运行时返回错误。

mod client;
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

pub use client::{FetchClient, UA};
pub use request::{FetchRequest, HEADER_UA};
pub use response::FetchResponse;
