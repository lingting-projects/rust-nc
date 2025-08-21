use crate::http;
use crate::http::ResponseExt;
use library_core::app::get_app;
use library_core::core::AnyResult;
use library_core::data_size::DataSize;
use library_nc::core::fast;
use serde::{Deserialize, Serialize};
use std::os::windows::process::CommandExt;
use std::process::{exit, Command};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct Asset {
    browser_download_url: String,
    name: String,
    size: u64,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

pub async fn check_async() -> AnyResult<Option<(String, String, DataSize)>> {
    if cfg!(debug_assertions) {
        return Ok(None);
    }

    let version = env!("CARGO_PKG_VERSION");
    let app = get_app();
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        app.owner, app.repo
    );

    let response = http::get(&url).await?;
    let json = response.read_text().await?;

    let release: Release = serde_json::from_str(&json)?;
    if !release.tag_name.eq(version) {
        let option = release
            .assets
            .into_iter()
            .find(|asset| asset.name.eq("Windows-lingting-nc.msi"));

        if let Some(asset) = option {
            let size = DataSize::of_bytes(asset.size);
            return Ok(Some((release.tag_name, asset.browser_download_url, size)));
        }
    }

    Ok(None)
}

pub fn check() -> AnyResult<Option<(String, String, DataSize)>> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .build()?;

    runtime.block_on(check_async())
}

pub struct UpdateListener {
    pub url: String,
    pub on_download: Box<dyn Fn()>,
    pub on_install: Box<dyn Fn()>,
}

pub fn update(listener: UpdateListener) -> AnyResult<()> {
    use hex::encode;
    use sha2::{Digest, Sha256};

    let url = listener.url;
    let sha256_bytes = Sha256::digest(&url);
    let sha256 = encode(sha256_bytes);
    let app = get_app();
    let path = &app.tmp_dir.join(&sha256);
    let fast = fast(&url);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .build()?;
    (listener.on_download)();
    runtime.block_on(async {
        let response = http::get(&fast).await.expect("文件下载请求异常!");
        response.overwrite(path).await
    })?;

    (listener.on_install)();
    let mut cmd = Command::new("msiexec");
    cmd.arg("/i")
        .arg(&path)
        .arg("/qb")
        // 独立新进程
        .creation_flags(0x00000200)
        .spawn()?;

    exit(0);
}
