use crate::core::{current_millis, AnyResult};
use crate::system;
use std::io::Read;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread::sleep;
use std::time::Duration;

pub struct Process {
    child: Child,
    pub charset: String,
    status: Option<ExitStatus>,
}

impl Process {
    pub fn new(cmd: Command) -> AnyResult<Self> {
        let charset = system::charset::get_system()?;
        Self::new_charset(cmd, charset)
    }

    pub fn new_charset(mut cmd: Command, charset: String) -> AnyResult<Self> {
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            // 在windows下, 不弹 cmd/powershell 创建
            cmd.creation_flags(0x08000000);
        }
        let child = cmd.spawn()?;
        Ok(Self {
            child,
            charset,
            status: None,
        })
    }
    pub fn new_pipe(cmd: Command) -> AnyResult<Self> {
        let charset = system::charset::get_system()?;
        Self::new_pipe_charset(cmd, charset)
    }

    pub fn new_pipe_charset(mut cmd: Command, charset: String) -> AnyResult<Self> {
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        Self::new_charset(cmd, charset)
    }

    pub fn status(&mut self) -> AnyResult<Option<ExitStatus>> {
        if self.status.is_some() {
            return Ok(self.status);
        }
        let option = self.child.try_wait()?;
        self.status = option;
        Ok(option)
    }

    pub fn wait(&mut self) -> AnyResult<()> {
        self.wait_millis(-1)?;
        Ok(())
    }

    /// 返回true表示在等待过程中结束, 返回false表示等待超时未结束
    pub fn wait_timeout(&mut self, duration: Duration) -> AnyResult<bool> {
        self.wait_millis(duration.as_millis() as i128)
    }

    /// 返回true表示在等待过程中结束, 返回false表示等待超时未结束
    pub fn wait_millis(&mut self, millis: i128) -> AnyResult<bool> {
        let start = current_millis()?;

        loop {
            let option = self.child.try_wait()?;

            if let Some(_) = option {
                self.status = option;
                return Ok(true);
            }

            // 存在超时判断
            if millis > 0 {
                let current = current_millis()?;
                let diff = current - start;
                if diff >= (millis as u128) {
                    return Ok(false);
                }
            }
            sleep(Duration::from_millis(100))
        }
    }

    pub fn kill(&mut self) -> AnyResult<()> {
        self.child.kill()?;
        Ok(())
    }

    pub fn out_bytes(&mut self) -> AnyResult<Option<Vec<u8>>> {
        if let Some(out) = &mut self.child.stdout {
            let mut buffer = Vec::new();
            out.read_to_end(&mut buffer)?;
            return Ok(Some(buffer));
        }

        Ok(None)
    }

    pub fn out_string(&mut self) -> AnyResult<Option<String>> {
        if let Some(vec) = self.out_bytes()? {
            let string = system::charset::convert(vec, &self.charset)?;
            return Ok(Some(string));
        }

        Ok(None)
    }

    pub fn err_bytes(&mut self) -> AnyResult<Option<Vec<u8>>> {
        if let Some(err) = &mut self.child.stderr {
            let mut buffer = Vec::new();
            err.read_to_end(&mut buffer)?;
            return Ok(Some(buffer));
        }

        Ok(None)
    }

    pub fn err_string(&mut self) -> AnyResult<Option<String>> {
        if let Some(vec) = self.err_bytes()? {
            let string = system::charset::convert(vec, &self.charset)?;
            return Ok(Some(string));
        }

        Ok(None)
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        match self.child.kill() {
            Ok(_) => {}
            Err(e) => {
                log::error!("结束子进程异常: {}", e);
            }
        }
    }
}
