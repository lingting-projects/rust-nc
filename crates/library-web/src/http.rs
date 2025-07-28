use library_core::app::APP;
use library_core::core::AnyResult;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client, ClientBuilder, Method, RequestBuilder, Response, Url};
use std::str::FromStr;
use std::sync::LazyLock;
use std::time::Duration;

static client: LazyLock<Client> = LazyLock::new(|| {
    let mut headers = HeaderMap::new();
    headers.insert("Accept", HeaderValue::from_str("*/*").unwrap());
    headers.insert(
        "Accept-Encoding",
        HeaderValue::from_str("gzip, deflate, br").unwrap(),
    );
    headers.insert(
        "Accept-Language",
        HeaderValue::from_str("en-US,en;q=0.5").unwrap(),
    );
    ClientBuilder::new()
        .timeout(Duration::from_secs(360))
        // 使用原生tls
        .use_native_tls()
        // 支持gzip
        .gzip(true)
        // 禁用ssl
        .danger_accept_invalid_certs(true)
        .danger_accept_invalid_hostnames(true)
        // 模拟浏览器
        .default_headers(headers)
        .build()
        .expect("http client new err")
});

fn build(method: Method, url: &str) -> AnyResult<RequestBuilder> {
    let mut builder = client.request(method, Url::from_str(url)?);
    let _ua = APP.get().map(|a| a.ua);
    if let Some(ua) = _ua {
        builder = builder.header("User-Agent", HeaderValue::from_str(ua)?);
    }

    Ok(builder)
}

pub async fn get(url: &str) -> AnyResult<Response> {
    let response = build(Method::GET, url)?.send().await?;

    match response.error_for_status() {
        Ok(r) => Ok(r),
        Err(e) => Err(Box::new(e)),
    }
}
