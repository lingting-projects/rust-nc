use crate::http::pick_host;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use library_core::core::AnyResult;
use std::clone::Clone;
use std::iter::Iterator;
use std::sync::LazyLock;

pub fn base64_decode(source: &str) -> AnyResult<String> {
    let vec = BASE64_STANDARD.decode(source)?;
    let string = String::from_utf8(vec)?;
    Ok(string)
}

#[derive(thiserror::Error, Debug)]
pub enum NcError {
    #[error("不支持的来源")]
    UnsupportedSource,
}

pub static PREFIX_REMAIN_TRAFFIC: LazyLock<Vec<String>> =
    LazyLock::new(|| vec!["剩余流量：".to_string()]);

pub static PREFIX_EXPIRE: LazyLock<Vec<String>> = LazyLock::new(|| vec!["套餐到期：".to_string()]);

pub static FAST_GItHUB_PREFIX: LazyLock<String> =
    LazyLock::new(|| "https://fastgh.lainbo.com/".to_string());

pub static FAST_GITHUB_KEYS: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec![
        "github".to_string(),
        "gist".to_string(),
        "githubusercontent".to_string(),
    ]
});

pub fn fast(url: &str) -> String {
    match pick_host(url) {
        None => url.to_string(),
        Some(h) => {
            if FAST_GITHUB_KEYS.iter().find(|k| h.contains(*k)).is_some() {
                return format!("{}{}", FAST_GItHUB_PREFIX.clone(), url);
            }

            url.to_string()
        }
    }
}

