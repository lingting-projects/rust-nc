//! 一次性子进程命令的共用部分。

use std::process::ExitStatus;

use anyhow::{Result, bail};

/// 检查子进程退出码，非零视为失败。
pub(super) fn check_status(status: ExitStatus) -> Result<()> {
    let code = status.code().unwrap_or(-1);
    if code == 0 {
        return Ok(());
    }
    bail!("内核执行失败, 退出码: {code}")
}
