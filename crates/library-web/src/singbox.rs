use crate::tbl_config::TblConfig;
use crate::tbl_setting::TblSettingRun;
use crate::{route_kernel, settings};
use library_core::core::{AnyResult, BizError};
use library_sing_box::{SingBox, State};
use std::path::Path;
use std::sync::{Arc, LazyLock, Mutex};

static INSTANCE: LazyLock<Arc<Mutex<Box<dyn SingBox + 'static + Send>>>> = LazyLock::new(|| {
    let r = library_sing_box::create();
    Arc::new(Mutex::new(r.expect("failed init instance")))
});

pub fn state() -> AnyResult<State> {
    match INSTANCE.lock() {
        Ok(mut x) => x.state(),
        Err(e) => {
            log::error!("获取sing box 实例异常! {}", e);
            Err(Box::new(BizError::SingBoxInit))
        }
    }
}

pub fn start(config_path: &Path, work_dir: &Path) -> AnyResult<()> {
    match INSTANCE.lock() {
        Ok(mut x) => x.start(config_path, work_dir),
        Err(e) => {
            log::error!("获取sing box 实例异常! {}", e);
            Err(Box::new(BizError::SingBoxInit))
        }
    }
}

pub fn stop() -> AnyResult<()> {
    match INSTANCE.lock() {
        Ok(mut x) => x.stop(),
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

pub(crate) fn init() -> AnyResult<()> {
    let run = TblSettingRun::get()?;
    if run.auto
        && let Some(config_id) = run.selected
        && !config_id.is_empty()
    {
        if let Some(config) = TblConfig::find(&config_id)? {
            route_kernel::start(config)?
        }
    }

    Ok(())
}
