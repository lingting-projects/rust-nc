use crate::core::AnyResult;
use regex::Regex;
use std::sync::LazyLock;

pub fn url_decode(source: &str) -> AnyResult<String> {
    let decode = percent_encoding::percent_decode_str(source);
    let cow = decode.decode_utf8()?;
    let string = cow.into_owned();
    Ok(string)
}

pub static PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^https?://(([a-zA-Z0-9.-]+)(:[0-9]+)?)(/.*)?$").unwrap());

pub fn pick_host(url: &str) -> Option<String> {
    PATTERN
        .captures(url)
        .and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()))
}
