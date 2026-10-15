use std::path::Path;
use std::process::{Command, ExitStatus};

use anyhow::{Context, Result, bail};

use crate::{BIN_NAME, SingBox, State};
use lib_system::process::Process;

/// 通过外部可执行文件运行 sing-box。
pub struct BinSingBox {
    bin: String,
    process: Option<Process>,
    error: bool,
    reason: Option<String>,
}

impl BinSingBox {
    /// 创建实例，`bin_dir` 为内核可执行文件所在目录。
    pub fn new(bin_dir: impl AsRef<Path>) -> Result<Self> {
        let bin = bin_dir.as_ref().join(BIN_NAME);
        let bin = bin
            .to_str()
            .with_context(|| format!("内核路径非 UTF-8: {}", bin.display()))?
            .to_string();
        log::info!("load sing box bin from {bin}");
        Ok(Self {
            bin,
            process: None,
            error: false,
            reason: None,
        })
    }

    /// 检查子进程退出码，非零视为失败。
    fn check(status: ExitStatus) -> Result<()> {
        let code = status.code().unwrap_or(-1);
        if code == 0 {
            return Ok(());
        }
        bail!("内核执行失败, 退出码: {code}")
    }
}

impl SingBox for BinSingBox {
    fn state(&mut self) -> Result<State> {
        if let Some(process) = &mut self.process {
            // 已经结束时判断结果, 未结束不做处理
            if let Some(status) = process.status()? {
                self.process = None;
                if let Err(error) = Self::check(status) {
                    log::error!("singbox执行异常! {error}");
                    self.error = true;
                    self.reason = Some(error.to_string());
                }
            }
        }

        Ok(State {
            running: self.process.is_some(),
            error: self.error,
            reason: self.reason.clone(),
        })
    }

    fn start(&mut self, config_path: &Path, work_dir: &Path) -> Result<()> {
        if self.process.is_some() {
            return Ok(());
        }

        let config = config_path
            .to_str()
            .with_context(|| format!("配置路径非 UTF-8: {}", config_path.display()))?;
        let work = work_dir
            .to_str()
            .with_context(|| format!("工作目录非 UTF-8: {}", work_dir.display()))?;

        let mut cmd = Command::new(&self.bin);
        cmd.arg("start")
            .current_dir(work_dir)
            .arg(config)
            .arg(work)
            // 父进程 pid, 内核据此判断是否需要退出
            .arg(std::process::id().to_string());

        self.process = Some(Process::new(cmd)?);
        self.error = false;
        self.reason = None;
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        let Some(mut process) = self.process.take() else {
            return Ok(());
        };

        match process.kill() {
            Ok(()) => Ok(()),
            Err(error) => {
                // 没结束, 放回去
                self.process = Some(process);
                log::error!("singbox结束进程异常! {error}");
                Err(error)
            }
        }
    }

    fn json_to_srs(&mut self, json_path: &Path, srs_path: &Path) -> Result<()> {
        let json = json_path
            .to_str()
            .with_context(|| format!("json 路径非 UTF-8: {}", json_path.display()))?;
        let srs = srs_path
            .to_str()
            .with_context(|| format!("srs 路径非 UTF-8: {}", srs_path.display()))?;

        let mut cmd = Command::new(&self.bin);
        cmd.arg("json2srs").arg(json).arg(srs);
        let status = cmd.status()?;
        Self::check(status)
    }
}

impl Drop for BinSingBox {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
