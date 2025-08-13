use crate::SingBox;
use library_core::core::{AnyResult, BizError};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::sync::{Arc, LazyLock, Mutex};

static BIN: LazyLock<String> = LazyLock::new(|| {
    let path = PathBuf::from(env!("SING_BOX_DIR")).join("lingting-nc-singbox");
    let ps = path.to_str().expect("failed load sing box bin path");
    log::info!("load sing box bin from {}", ps);
    ps.to_string()
});

pub struct BinSingBox {}

impl SingBox for BinSingBox {
    fn start(&self, config_path: &Path, work_dir: &Path) -> AnyResult<()> {
        let mut cmd = Command::new(BIN.clone());
        cmd.arg("start")
            .current_dir(work_dir)
            .arg(config_path.to_str().expect("failed get config path"))
            .arg(work_dir.to_str().expect("failed get work dir"));
        let status = cmd.status()?;
        check_result(status)
    }

    fn json_srs(&self, json_path: &Path, srs_path: &Path) -> AnyResult<()> {
        let mut cmd = Command::new(BIN.clone());
        cmd.arg("json2srs")
            .arg(json_path.to_str().expect("failed get json path"))
            .arg(srs_path.to_str().expect("failed get srs path"));
        let status = cmd.status()?;
        check_result(status)
    }
}

fn check_result(status: ExitStatus) -> AnyResult<()> {
    let i = status.code().unwrap_or(-999);
    if i >= 0 {
        Ok(())
    } else {
        Err(Box::new(BizError::OperationFailed(i)))
    }
}

pub fn new() -> AnyResult<BinSingBox> {
    Ok(BinSingBox {})
}
