//! 子进程封装：统一处理字符集解码、超时等待与生命周期回收。

use std::io::Read;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::Duration;

use anyhow::Result;

use crate::charset;

/// 已启动的子进程。
pub struct Process {
    child: Child,
    charset: String,
    status: Option<ExitStatus>,
}

impl Process {
    /// 启动子进程。
    pub fn new(cmd: Command) -> Result<Self> {
        let charset = charset::system()?;
        Self::with_charset(cmd, charset)
    }

    /// 启动子进程并收集输出。
    pub fn new_pipe(cmd: Command) -> Result<Self> {
        let charset = charset::system()?;
        Self::new_pipe_charset(cmd, charset)
    }

    /// 启动子进程并收集输出，指定输出解码使用的字符集。
    pub fn new_pipe_charset(mut cmd: Command, charset: String) -> Result<Self> {
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        Self::with_charset(cmd, charset)
    }

    /// 启动子进程，并指定输出解码使用的字符集。
    pub fn with_charset(mut cmd: Command, charset: String) -> Result<Self> {
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            // 在 windows 下，不弹出 cmd/powershell 窗口
            cmd.creation_flags(0x08000000);
        }
        let child = cmd.spawn()?;
        Ok(Self {
            child,
            charset,
            status: None,
        })
    }

    /// 查询子进程状态，未结束时返回 `None`。
    pub fn status(&mut self) -> Result<Option<ExitStatus>> {
        if self.status.is_some() {
            return Ok(self.status);
        }
        self.status = self.child.try_wait()?;
        Ok(self.status)
    }

    /// 等待指定时长；等待期间结束返回 `true`，超时返回 `false`。
    pub fn wait_timeout(&mut self, duration: Duration) -> Result<bool> {
        let millis = i128::from(duration.as_millis() as u64);
        self.wait_millis(millis)
    }

    /// 等待指定毫秒；等待期间结束返回 `true`，超时返回 `false`，`millis` 非正数表示无限等待。
    pub fn wait_millis(&mut self, millis: i128) -> Result<bool> {
        let start = lib_core::current_millis()?;

        loop {
            if let Some(status) = self.child.try_wait()? {
                self.status = Some(status);
                return Ok(true);
            }

            if millis > 0 && i128::from(lib_core::current_millis()?) - i128::from(start) >= millis {
                return Ok(false);
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// 读取标准输出。
    pub fn out_string(&mut self) -> Result<Option<String>> {
        read_string(&mut self.child.stdout, &self.charset)
    }

    /// 读取标准错误。
    pub fn err_string(&mut self) -> Result<Option<String>> {
        read_string(&mut self.child.stderr, &self.charset)
    }

    /// 结束子进程。
    pub fn kill(&mut self) -> Result<()> {
        self.child.kill()?;
        Ok(())
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        if let Err(error) = self.child.kill() {
            log::debug!("结束子进程异常: {error}");
        }
    }
}

/// 读取子进程的输出流。
fn read_string<R: Read>(reader: &mut Option<R>, charset: &str) -> Result<Option<String>> {
    let Some(reader) = reader else {
        return Ok(None);
    };
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
    Ok(Some(charset::convert(buffer, charset)?))
}
