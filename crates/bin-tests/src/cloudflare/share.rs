//! worker 分享接口的校验。
//!
//! `/s/{key}` 按环境变量 `share` 里的配置返回 307 重定向。校验需要看原始响应,
//! 不能被客户端跟随重定向, 因此这里直接走一条裸 HTTP 连接, 只读取状态行与响应头。

use std::io::{Read, Write};
use std::net::TcpStream;

use anyhow::{Context, Result, bail};
use serde_json::json;

/// 分享源环境变量名, 与 `bin-cloudflare` 保持一致。
pub const VARIABLE: &str = "share";

/// 分享键: 明文地址。
pub const KEY_PLAIN: &str = "plain";
/// 分享键: 带参数的地址。
pub const KEY_SECURE: &str = "secure";
/// 带参数地址的分享参数。
pub const SECURE_P: &str = "a1";
/// 明文地址的重定向目标。
pub const PLAIN_URL: &str = "https://github.com/lingting";
/// 带参数地址的重定向目标。
pub const SECURE_URL: &str = "https://github.com/lingting-projects";

/// 分享源配置内容, 值即环境变量 `share` 的取值。
///
/// `plain` 为明文地址, 任何请求都放行; `secure` 为带参数地址, 只有 `p` 匹配才放行。
pub fn config() -> String {
    let source = json!({
        KEY_PLAIN: PLAIN_URL,
        KEY_SECURE: { "p": SECURE_P, "u": SECURE_URL },
    });

    source.to_string()
}

/// 原始响应。
pub struct Response {
    pub status: u16,
    headers: Vec<(String, String)>,
}

impl Response {
    /// 读取响应头, 名称不区分大小写。
    pub fn header(&self, name: &str) -> Option<&str> {
        let name = name.to_ascii_lowercase();
        self.headers
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value.as_str())
    }
}

/// 向 worker 发送一次 GET 请求并读取原始响应。
pub fn get(base: &str, path: &str) -> Result<Response> {
    let address = base
        .strip_prefix("http://")
        .with_context(|| format!("不支持的地址: {base}"))?;

    let mut stream =
        TcpStream::connect(address).with_context(|| format!("连接 worker 失败: {address}"))?;
    let request = format!("GET {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes())?;

    let mut source = String::new();
    stream.read_to_string(&mut source)?;
    parse(&source)
}

/// 解析状态行与响应头。
fn parse(source: &str) -> Result<Response> {
    let mut lines = source.split("\r\n");
    let status = lines
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse().ok())
        .context("响应缺少状态行")?;

    let mut headers = Vec::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        headers.push((name.to_ascii_lowercase(), value.trim().to_string()));
    }

    Ok(Response { status, headers })
}

/// 校验重定向: 状态码为 307 且地址与预期一致。
pub fn expect_redirect(response: &Response, target: &str) -> Result<()> {
    if response.status != 307 {
        bail!("分享应返回 307, 实际为 {}", response.status);
    }

    match response.header("location") {
        Some(location) if location == target => Ok(()),
        Some(location) => bail!("重定向地址应为 {target}, 实际为 {location}"),
        None => bail!("重定向缺少 location 响应头"),
    }
}

/// 校验拒绝: 分享键或参数不匹配时返回 403。
pub fn expect_deny(response: &Response) -> Result<()> {
    if response.status != 403 {
        bail!("应拒绝访问(403), 实际为 {}", response.status);
    }

    Ok(())
}
