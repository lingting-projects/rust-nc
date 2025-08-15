use library_core::app::app_map;
use library_core::core::AnyResult;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client, ClientBuilder, Method, RequestBuilder, Response, Url};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::str::FromStr;
use std::sync::LazyLock;
use std::time::Duration;

pub trait ResponseExt {
    async fn read_text(self) -> AnyResult<String>;
    async fn overwrite<P: AsRef<Path>>(self, p: P) -> AnyResult<()>;
}

impl ResponseExt for Response {
    async fn read_text(self) -> AnyResult<String> {
        let text = self.text().await?;
        Ok(text)
    }

    async fn overwrite<P: AsRef<Path>>(mut self, p: P) -> AnyResult<()> {
        let mut file = File::options()
            .write(true)
            .create(true)
            .truncate(true)
            .open(p)?;
        while let Some(bytes) = self.chunk().await? {
            file.write_all(&bytes)?;
            file.flush()?;
        }

        Ok(())
    }
}

static client: LazyLock<Client> = LazyLock::new(|| {
    let mut headers = HeaderMap::new();
    headers.insert("Accept-Encoding", HeaderValue::from_str("gzip").unwrap());
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
    if let Some(ua) = app_map(|a| a.ua) {
        builder = builder.header("User-Agent", HeaderValue::from_str(ua)?);
    }

    Ok(builder)
}

fn handler(response: Response) -> AnyResult<Response> {
    match response.error_for_status() {
        Ok(r) => Ok(r),
        Err(e) => Err(Box::new(e)),
    }
}

pub async fn get(url: &str) -> AnyResult<Response> {
    let response = build(Method::GET, url)?.send().await?;
    handler(response)
}
