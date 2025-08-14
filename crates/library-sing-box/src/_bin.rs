use crate::{SingBox, State};
use library_core::core::{AnyResult, BizError};
use library_core::system;
use library_core::system::process::Process;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus};
use std::sync::{Arc, LazyLock, Mutex};

static BIN: LazyLock<String> = LazyLock::new(|| {
    let path = PathBuf::from(env!("SING_BOX_DIR")).join("lingting-nc-singbox");
    let ps = path.to_str().expect("failed load sing box bin path");
    log::info!("load sing box bin from {}", ps);
    ps.to_string()
});

pub struct BinSingBox {
    process: Option<Process>,
    error: bool,
    reason: Option<String>,
}

impl SingBox for BinSingBox {
    fn state(&mut self) -> AnyResult<State> {
        if let Some(process) = &mut self.process {
            match process.status() {
                Ok(Some(s)) => {
                    self.process = None;
                    // 已经结束, 判断结果
                    if let Err(e) = check_result(s) {
                        log::error!("singbox执行异常! {}", &e);
                        self.error = true;
                        self.reason = Some(e.to_string());
                    }
                }
                Ok(None) => {
                    // 未结束, 不做处理
                }
                Err(e) => {
                    log::error!("获取singbox运行状态异常! {}", e)
                }
            }
        }

        Ok(State {
            running: self.process.is_some(),
            error: self.error,
            reason: self.reason.clone(),
        })
    }

    fn start(&mut self, config_path: &Path, work_dir: &Path) -> AnyResult<()> {
        if self.process.is_some() {
            return Ok(());
        }

        let mut cmd = Command::new(BIN.clone());
        cmd.arg("start")
            .current_dir(work_dir)
            .arg(config_path.to_str().expect("failed get config path"))
            .arg(work_dir.to_str().expect("failed get work dir"));

        let process = Process::new(cmd)?;

        self.process = Some(process);
        self.error = false;
        self.reason = None;
        Ok(())
    }

    fn stop(&mut self) -> AnyResult<()> {
        let option = self.process.take();
        if let Some(mut c) = option {
            match c.kill() {
                Ok(_) => {}
                Err(e) => {
                    // 没结束, 放回去
                    self.process = Some(c);
                    log::error!("singbox结束进程异常! {}", e)
                }
            }
        }
        Ok(())
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
    Ok(BinSingBox {
        process: None,
        error: false,
        reason: None,
    })
}
