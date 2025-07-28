use library_core::app::APP;
use library_core::core::AnyResult;
use reqwest::header::HeaderValue;
use reqwest::{Client, ClientBuilder, Method, RequestBuilder, Response, Url};
use std::str::FromStr;
use std::sync::LazyLock;
use std::time::Duration;

static client: LazyLock<Client> = LazyLock::new(|| {
    ClientBuilder::new()
        .timeout(Duration::from_secs(360))
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
