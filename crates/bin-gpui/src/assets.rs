use std::borrow::Cow;
use std::path::{Path, PathBuf};

use gpui_kit::assets::{AllAssets, Assets};
use gpui_kit::*;

/// 应用资源源。
pub struct AppAssets {
    assets: Assets,
    all: AllAssets,
    /// 磁盘上的 `assets/` 目录，仅作为回退源。
    root: PathBuf,
}

impl AppAssets {
    /// 资源目录位于仓库根目录的 `assets/`。
    pub fn new() -> Self {
        Self {
            assets: Assets,
            all: AllAssets,
            root: Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets"),
        }
    }
}

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
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
        names.sort();
        names.dedup();
        Ok(names)
    }
}
