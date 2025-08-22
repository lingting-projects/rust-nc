use crate::http;
use crate::http::ResponseExt;
use library_core::app::get_app;
use library_core::core::AnyResult;
use library_core::data_size::DataSize;
use library_nc::core::fast;
use serde::{Deserialize, Serialize};
use std::os::windows::process::CommandExt;
use std::process::{exit, Command};
use std::sync::{LazyLock, Mutex};

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

struct Version {
    major: u16,
    minor: u16,
    patch: u16,
}

impl Version {
    pub fn resolver(str: &str) -> Option<Self> {
        // 按点分割版本号组件
        let parts: Vec<&str> = str.split('.').collect();

        // 检查组件数量是否正确
        if parts.len() != 3 {
            return None;
        }

        let major = parts[0].parse().ok();
        let minor = parts[1].parse().ok();
        let patch = parts[2].parse().ok();

        if major.is_none() || minor.is_none() || patch.is_none() {
            return None;
        }

        Some(Version {
            major: major.unwrap(),
            minor: minor.unwrap(),
            patch: patch.unwrap(),
        })
    }

    /// 当前版本是否大于目标版本
    pub fn is_gt(&self, t: &Self) -> bool {
        self.major > t.major || self.minor > t.minor || self.patch > t.patch
    }
}

pub async fn check_async() -> AnyResult<Option<(String, String, DataSize)>> {
    if cfg!(debug_assertions) {
        return Ok(None);
    }

    let version = Version::resolver(env!("CARGO_PKG_VERSION")).unwrap();
    let app = get_app();
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        app.owner, app.repo
    );

    let response = http::get(&url).await?;
    let json = response.read_text().await?;

    let release: Release = serde_json::from_str(&json)?;
    let last = Version::resolver(&release.tag_name);
    if let Some(v) = last {
        if v.is_gt(&version) {
            let option = release
                .assets
                .into_iter()
                .find(|asset| asset.name.eq("Windows-lingting-nc.msi"));

            if let Some(asset) = option {
                let size = DataSize::of_bytes(asset.size);
                return Ok(Some((release.tag_name, asset.browser_download_url, size)));
            }
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

    let guard = _exit.lock().unwrap();
    guard(0);
    Ok(())
}

static _exit: LazyLock<Mutex<Box<dyn Fn(i32) + 'static + Send + Sync>>> =
    LazyLock::new(|| Mutex::new(Box::new(|i| exit(i))));

pub fn set_exit<F: Fn(i32) + 'static + Send + Sync>(f: F) {
    *_exit.lock().unwrap() = Box::new(f)
}
