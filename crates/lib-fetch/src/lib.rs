//! 远程资源获取。
//!
//! 统一使用固定 UA 的浏览器请求, 并跟随重定向。

pub mod url;

use std::sync::LazyLock;
use std::time::Duration;

use anyhow::Result;
use form_urlencoded::Serializer;
use wreq::Client;
use wreq::header::{ACCEPT_LANGUAGE, CONTENT_TYPE, HeaderMap, HeaderValue, USER_AGENT};
use wreq::redirect::Policy;

/// 请求使用的 User-Agent。
pub const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/135.0.0.0 Safari/537.36";

/// 请求超时时间。
const TIMEOUT: Duration = Duration::from_secs(360);

/// 跟随的最大重定向次数。
const REDIRECT_LIMIT: usize = 10;

static CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .timeout(TIMEOUT)
        .redirect(Policy::limited(REDIRECT_LIMIT))
        .gzip(true)
        .brotli(true)
        .zstd(true)
        .deflate(true)
        .default_headers(headers())
        // 订阅源经常使用自签名证书
        .tls_cert_verification(false)
        .tls_verify_hostname(false)
        .build()
        .expect("http client 初始化失败")
});

fn headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(UA));
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.5"));
    headers
}

/// 发送 GET 请求并返回响应文本。
pub async fn fetch_get(url: &str) -> Result<String> {
    let response = CLIENT.get(url).send().await?.error_for_status()?;
    Ok(response.text().await?)
}

/// 以原始字符串作为请求体发送 POST 请求并返回响应文本。
pub async fn fetch_post(url: &str, body: &str) -> Result<String> {
    let response = CLIENT
        .post(url)
        .body(body.to_string())
        .send()
        .await?
        .error_for_status()?;
    Ok(response.text().await?)
}

/// 以表单作为请求体发送 POST 请求并返回响应文本。
pub async fn fetch_post_form(url: &str, form: &[(&str, &str)]) -> Result<String> {
    let mut serializer = Serializer::new(String::new());
    for (key, value) in form {
        serializer.append_pair(key, value);
    }
    let body = serializer.finish();

    let response = CLIENT
        .post(url)
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await?
        .error_for_status()?;
    Ok(response.text().await?)
}
