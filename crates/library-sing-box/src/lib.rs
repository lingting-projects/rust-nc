#[cfg(feature = "bin")]
mod _bin;
#[cfg(not(feature = "bin"))]
mod _lib;

use library_core::core::{AnyResult, BizError};
use std::path::Path;
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};

pub static version: &'static str = "v1.11.9";

pub trait SingBox {
    fn start(&self, config_path: &Path, work_dir: &Path) -> AnyResult<()>;
    fn json_srs(&self, json_path: &Path, srs_path: &Path) -> AnyResult<()>;
}

static INSTANCE: LazyLock<Arc<Mutex<Box<dyn SingBox + Send + 'static>>>> = LazyLock::new(|| {
    #[cfg(feature = "bin")]
    let i = _bin::new();
    #[cfg(not(feature = "bin"))]
    let i = _lib::new();
    Arc::new(Mutex::new(Box::new(i.expect("failed init instance"))))
});

pub fn start(config_path: &Path, work_dir: &Path) -> AnyResult<()> {
    match INSTANCE.lock() {
        Ok(x) => x.start(config_path, work_dir),
        Err(e) => {
            log::error!("获取sing box 实例异常! {}", e);
            Err(Box::new(BizError::SingBoxInit))
        }
    }
}

pub fn json_to_srs(json_path: &Path, srs_path: &Path) -> AnyResult<()> {
    match INSTANCE.lock() {
        Ok(x) => x.json_srs(json_path, srs_path),
        Err(e) => {
            log::error!("获取sing box 实例异常! {}", e);
            Err(Box::new(BizError::SingBoxInit))
        }
    }
}
