//! 基于 `wreq` 的实现, 模拟 chrome 的 tls 指纹。

use std::sync::LazyLock;
use std::time::Duration;

use anyhow::Result;
use lib_core::MultiStringValue;
use wreq::Client;
use wreq::header::HeaderMap;
use wreq::redirect::Policy;
use wreq_util::{Emulation, Platform, Profile};

use crate::request::FetchRequest;
use crate::response::FetchResponse;
use crate::util::{pairs, to_multi_string_value};

/// 请求超时时间。
const TIMEOUT: Duration = Duration::from_secs(360);

/// 跟随的最大重定向次数。
const REDIRECT_LIMIT: usize = 10;

/// 模拟的 chrome 版本。
const PROFILE: Profile = Profile::Chrome135;

/// 模拟的操作系统。
const PLATFORM: Platform = Platform::Windows;

static CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .emulation(
            Emulation::builder()
                .profile(PROFILE)
                .platform(PLATFORM)
                .build(),
        )
        .timeout(TIMEOUT)
        .redirect(Policy::limited(REDIRECT_LIMIT))
        .gzip(true)
        .brotli(true)
        .zstd(true)
        .deflate(true)
        // 订阅源经常使用自签名证书
        .tls_cert_verification(false)
        .tls_verify_hostname(false)
        .build()
        .expect("http client 初始化失败")
});

/// 发送 GET 请求。
pub(crate) async fn get(request: FetchRequest) -> Result<FetchResponse> {
    send(CLIENT.get(&request.url), &request).await
}

/// 发送 POST 请求。
pub(crate) async fn post(request: FetchRequest) -> Result<FetchResponse> {
    send(CLIENT.post(&request.url), &request).await
}

/// 写入请求头与请求体后发送请求。
async fn send(mut builder: wreq::RequestBuilder, request: &FetchRequest) -> Result<FetchResponse> {
    builder = builder.headers(headers(&request.headers));
    if let Some(body) = &request.body {
        builder = builder.body(body.clone());
    }

    let response = builder.send().await?;
    let status = response.status().as_u16();
    let headers = to_multi_string_value(
        response
            .headers()
            .iter()
            .map(|(name, value)| (name.as_str(), String::from_utf8_lossy(value.as_bytes()))),
    );
    let body = response.bytes().await?;

    Ok(FetchResponse::new(body, headers, status))
}

/// [`MultiStringValue`] 转请求头, 同名请求头保留多条。
fn headers(source: &MultiStringValue) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for (name, value) in pairs(source) {
        if let (Ok(name), Ok(value)) = (
            name.parse::<wreq::header::HeaderName>(),
            value.parse::<wreq::header::HeaderValue>(),
        ) {
            headers.append(name, value);
        }
    }
    headers
}
