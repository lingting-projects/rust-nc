//! 代理连通性探测。

use anyhow::{Result, bail};
use lib_fetch::{FetchClient, FetchRequest};

/// 探测目标: 作者主页, 只取响应头。
pub const TARGET: &str = "https://github.com/lingting";

/// 走代理向探测目标发送 head 请求, 状态码不是 2xx 即失败。
///
/// 只取响应头, 不下载正文; 请求经代理转发, 因此成功即说明代理链路可用。
pub async fn head(proxy: &str) -> Result<()> {
    let client = FetchClient::with_proxy(proxy)?;
    let response = client.head(FetchRequest::new(TARGET)).await?;
    if !response.is_success() {
        bail!(
            "代理 {proxy} 访问 {TARGET} 失败, 状态码: {}",
            response.status
        );
    }

    log::info!(
        "代理 {proxy} 访问 {TARGET} 成功, 状态码: {}",
        response.status
    );
    Ok(())
}
