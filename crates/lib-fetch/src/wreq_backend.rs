use std::sync::LazyLock;
use std::time::Duration;

use anyhow::Result;
use bytes::Bytes;
use futures_util::StreamExt as _;
use lib_core::MultiStringValue;
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

pub(crate) type Client = wreq::Client;

static CLIENT: LazyLock<Client> = LazyLock::new(|| create(None).expect("http client 初始化失败"));

/// 共享的默认客户端, 复用同一连接池。
pub(crate) fn shared() -> Client {
    CLIENT.clone()
}

/// 构建后端客户端, `proxy` 非空时所有请求走该代理。
pub(crate) fn create(proxy: Option<&str>) -> Result<Client> {
    let builder = Client::builder()
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
        .tls_verify_hostname(false);

    match proxy {
        Some(proxy) => builder.proxy(wreq::Proxy::all(proxy)?),
        None => builder,
    }
    .build()
    .map_err(Into::into)
}

/// 发送 GET 请求。
pub(crate) async fn get(client: &Client, request: FetchRequest) -> Result<FetchResponse> {
    send(client.get(&request.url), &request).await
}

/// 发送 POST 请求。
pub(crate) async fn post(client: &Client, request: FetchRequest) -> Result<FetchResponse> {
    send(client.post(&request.url), &request).await
}

/// 发送 HEAD 请求，只取响应头。
pub(crate) async fn head(client: &Client, request: FetchRequest) -> Result<FetchResponse> {
    send(client.head(&request.url), &request).await
}

/// 以 GET 请求取得响应体流。
///
/// 响应体不做整体收集，由调用方逐块消费，因此大文件不会一次性进内存。
pub(crate) async fn get_stream(client: &Client, request: FetchRequest) -> Result<FetchStream> {
    let builder = client.get(&request.url).headers(headers(&request.headers));
    let response = builder.send().await?;

    let status = response.status().as_u16();
    let headers = to_multi_string_value(
        response
            .headers()
            .iter()
            .map(|(name, value)| (name.as_str(), String::from_utf8_lossy(value.as_bytes()))),
    );

    Ok(FetchStream {
        status,
        headers,
        body: Box::pin(response.bytes_stream()),
    })
}

/// 流式响应：状态码、响应头与逐块到达的响应体。
pub(crate) struct FetchStream {
    status: u16,
    headers: MultiStringValue,
    body: std::pin::Pin<Box<dyn futures_util::Stream<Item = wreq::Result<Bytes>> + Send>>,
}

impl FetchStream {
    /// 是否请求成功。
    pub(crate) fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// 状态码。
    pub(crate) fn status(&self) -> u16 {
        self.status
    }

    /// 读取一个响应头，返回第一个值。
    pub(crate) fn header(&self, name: &str) -> Option<&String> {
        self.headers.get_first(name)
    }

    /// 读取下一块响应体，结束或出错时返回 `None`。
    pub(crate) async fn next(&mut self) -> Option<Result<Bytes>> {
        let chunk = self.body.next().await?;
        Some(chunk.map_err(Into::into))
    }
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

/// [`MultiStringValue`] 转请求头，同名请求头保留多条。
fn headers(source: &MultiStringValue) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for (name, value) in pairs(source) {
        if let (Ok(name), Ok(value)) = (
            name.parse::<wreq::header::HeaderName>(),
            name.parse::<wreq::header::HeaderValue>(),
        ) {
            headers.append(name, value);
        }
    }
    headers
}
