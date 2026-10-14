//! github 代理。
//!
//! 路径形如 `/gist/{owner}/{repo}/{path}`、`/release/{owner}/{repo}/{tag}/{file}`,
//! `/latest` 会先查询最新 release 的 tag 再转发到下载地址。

use anyhow::{Result, anyhow};
use lib_fetch::{FetchClient, FetchRequest};
use serde::Deserialize;
use worker::{Request, Response, console_debug, console_error};

use crate::core::{forward_headers, response_forwardable};

/// 支持的 owner。
const OWNERS: &[&str] = &["lingting", "lingting-projects"];

/// release 信息。
#[derive(Debug, Default, Deserialize)]
struct Release {
    tag_name: String,
}

/// 由请求路径解析出的代理参数。
struct Params {
    owner: String,
    repo: String,
    /// 路径中 owner、repo 之后的部分
    last: String,
}

impl Params {
    /// 由请求路径解析, 路径不足或 owner 不支持时返回 `None`。
    fn from_path(path: &str) -> Option<Self> {
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        if segments.len() < 4 {
            console_error!("path length err! {}", segments.len());
            return None;
        }

        let owner = segments[1].to_string();
        if !OWNERS.contains(&owner.as_str()) {
            console_error!("unsupported owner: {}", owner);
            return None;
        }

        let repo = segments[2].to_string();
        let last = segments[3..].join("/");
        console_debug!("owner: {}, repo: {}, last: {}", owner, repo, last);

        Some(Self { owner, repo, last })
    }

    /// gist 原始内容地址。
    fn gist(&self) -> String {
        format!(
            "https://gist.githubusercontent.com/{}/{}/raw/{}",
            self.owner, self.repo, self.last
        )
    }

    /// release 资源下载地址, `last` 已包含 tag 与文件名。
    fn release(&self) -> String {
        format!(
            "https://github.com/{}/{}/releases/download/{}",
            self.owner, self.repo, self.last
        )
    }

    /// 指定 tag 的 release 资源下载地址。
    fn release_with_tag(&self, tag: &str) -> String {
        format!(
            "https://github.com/{}/{}/releases/download/{}/{}",
            self.owner, self.repo, tag, self.last
        )
    }

    /// 最新 release 查询地址。
    fn latest(&self) -> String {
        format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            self.owner, self.repo
        )
    }
}

/// 代理 gist 原始内容。
pub async fn gist(req: &Request) -> Option<Result<Response>> {
    let params = Params::from_path(&req.path())?;
    Some(forward(req, &params.gist()).await)
}

/// 代理 release 资源。
pub async fn release(req: &Request) -> Option<Result<Response>> {
    let params = Params::from_path(&req.path())?;
    Some(forward(req, &params.release()).await)
}

/// 代理最新 release 资源。
pub async fn release_latest(req: &Request) -> Option<Result<Response>> {
    let params = Params::from_path(&req.path())?;

    let client = FetchClient::new();
    let url = params.latest();
    let response = match client
        .get(FetchRequest::new(&url).with_headers(forward_headers(req.headers())))
        .await
    {
        Ok(response) => response,
        Err(e) => return Some(Err(e)),
    };

    let source = response.string();
    if !response.is_success() {
        console_error!("github api error code: {}", response.status);
        console_error!("github api error body: {}", source);
        return None;
    }

    let release = match serde_json::from_str::<Release>(&source) {
        Ok(release) => release,
        Err(e) => return Some(Err(anyhow!(e))),
    };
    console_debug!("tag: {}", release.tag_name);

    Some(forward(req, &params.release_with_tag(&release.tag_name)).await)
}

/// 转发请求到 `url`, 并把响应返回给来源。
async fn forward(req: &Request, url: &str) -> Result<Response> {
    console_debug!("转发请求: {}", url);
    let request = FetchRequest::new(url).with_headers(forward_headers(req.headers()));
    let response = FetchClient::new().get(request).await?;

    let mut builder = Response::builder().with_status(response.status);
    let mut failure = None;
    response.headers.for_each(|name, values| {
        if !response_forwardable(name) {
            return;
        }
        for value in values {
            match builder.clone().with_header(name, value) {
                Ok(next) => builder = next,
                Err(e) => failure = Some((name.clone(), e.to_string())),
            }
        }
    });
    if let Some((name, reason)) = failure {
        console_error!("响应头[{}]写入失败! {}", name, reason);
    }

    Ok(builder.fixed(response.body.to_vec()))
}
