use worker::{Fetch, Method, Request, Response, Result};

pub async fn http_get(url: &str) -> Result<Response> {
    let request = Request::new(url, Method::Get)?;
    let fetch = Fetch::Request(request);
    fetch.send().await
}
