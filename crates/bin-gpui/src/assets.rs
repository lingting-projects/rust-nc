use std::borrow::Cow;
use std::path::{Path, PathBuf};

use gpui_kit::assets::{AllAssets, Assets};
use gpui_kit::*;

use lingting_gpui_kit::region::RegionAssets;

/// 应用资源源。
pub struct AppAssets {
    assets: Assets,
    all: AllAssets,
    /// 磁盘上的 `assets/` 目录，作为内嵌资源未命中时的回退源。
    root: PathBuf,
}

impl AppAssets {
    /// 优先使用可执行文件同级的 `assets/`，发布包即按该结构分发；
    /// 开发期可执行文件在 `target/{profile}` 下，回退到仓库根目录的 `assets/`。
    pub fn new() -> Self {
        Self {
            assets: Assets,
            all: AllAssets,
            root: assets_root(),
        }
    }
}

/// 解析磁盘资源目录。
fn assets_root() -> PathBuf {
    let installed = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.join("assets")));
    if let Some(installed) = installed
        && installed.is_dir()
    {
        return installed;
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

/// 磁盘资源目录。
///
/// 托盘图标要交给系统直接读文件，不走 [`AssetSource`]，因此单独暴露这条解析规则，
/// 与内嵌资源的回退顺序保持一致。
pub fn root() -> PathBuf {
    assets_root()
}

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        // 区域旗帜是内嵌 SVG，只从 `flags/` 取，避免与磁盘资源目录重名。
        if let Ok(Some(data)) = RegionAssets.load(path) {
            return Ok(Some(data));
        }
        // 找不到资源时内建源返回 `Err`，这里统一按「未命中」处理并继续回退。
        if let Ok(Some(data)) = self.assets.load(path) {
            return Ok(Some(data));
        }
        if let Ok(Some(data)) = self.all.load(path) {
            return Ok(Some(data));
        }
        Ok(std::fs::read(self.root.join(path)).ok().map(Cow::Owned))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut names = self.all.list(path).unwrap_or_default();
        names.extend(self.assets.list(path).unwrap_or_default());
        names.extend(RegionAssets.list(path).unwrap_or_default());
        names.sort();
        names.dedup();
        Ok(names)
    }
}
