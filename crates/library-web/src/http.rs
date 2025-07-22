use library_core::core::AnyResult;
use reqwest::Response;

pub async fn get(url: &str) -> AnyResult<Response> {
    match reqwest::get(url).await?.error_for_status() {
        Ok(r) => Ok(r),
        Err(e) => Err(Box::new(e)),
    }
}
