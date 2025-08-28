use crate::core::http_get;
use library_core::core::AnyResult;
use serde::{Deserialize, Serialize};
use worker::{console_debug, console_error, console_warn, Request, Response};

struct Params {
    owner: String,
    repo: String,
    last: String,
}

impl Params {
    pub fn from_fetch(req: Request) -> Option<Self> {
        let path = req.path();
        let vec: Vec<&str> = path.split("/").filter(|s| !s.is_empty()).collect();
        if vec.len() < 4 {
            console_warn!("path length err! {}", vec.len());
            return None;
        }
        let owner = vec.get(1).expect("unknown owner").to_string();
        console_debug!("owner: {owner}");

        if owner != "lingting" && owner != "lingting-projects" {
            console_warn!("unsupported owner: {owner}");
            return None;
        }

        let repo = vec.get(2).expect("unknown repo").to_string();
        console_debug!("repo: {repo}");
        let mut last = String::new();
        let mut i = -1;
        for x in vec {
            i += 1;
            if i < 3 {
                continue;
            }
            if last.is_empty() {
                last = x.to_string()
            } else {
                last = format!("{}/{}", last, x)
            }
        }

        console_debug!("last: {last}");
        if last.is_empty() {
            console_warn!("unsupported last: {last}");
            return None;
        }

        Some(Self { owner, repo, last })
    }
}

pub async fn gist(req: Request) -> Option<AnyResult<Response>> {
    let params = match Params::from_fetch(req) {
        None => return None,
        Some(v) => v,
    };

    let owner = params.owner;
    let repo = params.repo;
    let last = params.last;
    let url = format!("https://gist.githubusercontent.com/{owner}/{repo}/raw/{last}");
    Some(http_get(&url).await)
}

pub async fn release(req: Request) -> Option<AnyResult<Response>> {
    let params = match Params::from_fetch(req) {
        None => return None,
        Some(v) => v,
    };

    let owner = params.owner;
    let repo = params.repo;
    let last = params.last;
    let url = format!("https://github.com/{owner}/{repo}/releases/download/{last}");
    Some(http_get(&url).await)
}
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct Release {
    tag_name: String,
}

pub async fn release_latest(req: Request) -> Option<AnyResult<Response>> {
    let params = match Params::from_fetch(req) {
        None => return None,
        Some(v) => v,
    };

    let owner = params.owner;
    let repo = params.repo;
    let last = params.last;

    let latest = format!("https://api.github.com/repos/{owner}/{repo}/releases/latest",);
    let tag = match http_get(&latest).await {
        Ok(mut r) => {
            if r.status_code() != 200 {
                console_error!("github api error code: {}", r.status_code());
                match r.text().await {
                    Ok(text) => {
                        console_error!("github api error body: {}", text);
                    }
                    Err(_) => {}
                }
                return None;
            }
            match r.text().await {
                Ok(json) => {
                    let sr: serde_json::Result<Release> = serde_json::from_str(&json);
                    match sr {
                        Ok(release) => release.tag_name,
                        Err(e) => return Some(Err(Box::new(e))),
                    }
                }
                Err(e) => return Some(Err(Box::new(e))),
            }
        }
        Err(e) => return Some(Err(e)),
    };
    console_debug!("tag: {tag}");
    let url = format!("https://github.com/{owner}/{repo}/releases/download/{tag}/{last}");
    Some(http_get(&url).await)
}
