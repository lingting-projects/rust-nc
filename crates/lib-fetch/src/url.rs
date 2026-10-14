//! 远程地址工具。

use std::sync::LazyLock;

use lib_core::constants;
use regex::Regex;

static PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^https?://(([a-zA-Z0-9.-]+)(:[0-9]+)?)(/.*)?$").unwrap());

/// 提取 url 中的域名与端口。
pub fn pick_host(url: &str) -> Option<String> {
    PATTERN
        .captures(url)
        .and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()))
}

/// github 相关地址添加加速前缀。
pub fn fast(url: &str) -> String {
    let Some(host) = pick_host(url) else {
        return url.to_string();
    };

    if GITHUB_KEYS.iter().any(|key| host.contains(key)) {
        return format!("{}{}", constants::FAST_GITHUB_PREFIX, url);
    }

    url.to_string()
}

/// github 相关域名关键字。
static GITHUB_KEYS: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec![
        "github".to_string(),
        "gist".to_string(),
        "githubusercontent".to_string(),
    ]
});
