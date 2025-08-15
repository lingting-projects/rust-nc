use crate::http;
use crate::http::ResponseExt;
use library_core::app::get_app;
use library_core::core::AnyResult;
use library_core::data_size::DataSize;
use library_nc::core::fast;
use serde::{Deserialize, Serialize};

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

pub fn check() -> AnyResult<Option<(String, String, DataSize)>> {
    if cfg!(any(debug_assertions, not(target_os = "windows"))) {
        return Ok(None);
    }
    let version = env!("CARGO_PKG_VERSION");
    let app = get_app();
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        app.owner, app.repo
    );
    let fast = fast(&url);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .build()?;

    let json = runtime.block_on(async {
        let r = http::get(&fast).await;
        let response = r.expect(&format!("failed request: {}", url));
        return response.read_text().await;
    })?;

    let release: Release = serde_json::from_str(&json)?;
    if !release.tag_name.eq(version) {
        let option = release
            .assets
            .into_iter()
            .find(|asset| asset.name.eq("lingting-nc.exe"));

        if let Some(asset) = option {
            let size = DataSize::of_bytes(asset.size);
            return Ok(Some((release.tag_name, asset.browser_download_url, size)));
        }
    }

    Ok(None)
}
