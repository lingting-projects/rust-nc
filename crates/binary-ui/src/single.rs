use fs2::FileExt;
use library_core::core::AnyResult;
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::{fs, process};

pub struct Single {
    pub is_single: bool,
    pub pid: Option<u32>,
    pub info: String,
    pub path: String,
    _file: Option<File>,
}

fn try_unique<P: AsRef<Path>>(p: P) -> AnyResult<File> {
    let mut options = OpenOptions::new();
    options.write(true).create(true);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options
            .share_mode(0x7)
            .attributes(0)
            .security_qos_flags(0x0)
            .custom_flags(0x0)
            .access_mode(0xC0000000);
    }
    #[cfg(not(target_os = "windows"))]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // 所有人可以读写
        options.mode(0o666);
    }

    let file = options.open(p)?;
    file.lock_exclusive()?;
    Ok(file)
}

impl Single {
    pub fn create<P: AsRef<Path>>(p: P, info: &str) -> AnyResult<Single> {
        let path = p.as_ref();
        if let Some(parent) = path.parent() {
            create_dir_all(parent)?;
        }
        let p_str = path.to_str().expect("get path err").to_string();

        match try_unique(path) {
            Ok(mut file) => {
                let pid = process::id();
                let content = format!("{}\n{}", pid, info);

                // 写入PID和信息
                file.set_len(0)?; // 截断文件
                file.write_all(content.as_bytes())?;
                file.flush()?;
                file.sync_all()?;

                return Ok(Single {
                    is_single: true,
                    pid: Some(pid),
                    info: info.to_string(),
                    path: p_str,
                    _file: Some(file),
                });
            }
            Err(e) => {
                log::error!("获取独占锁异常! {}", e)
            }
        }

        let mut options = OpenOptions::new();
        options.read(true);

        let mut file = options.open(path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        let (pid_str, info) = content.split_once('\n').unwrap_or(("0", &content));
        let pid = pid_str.parse::<u32>().ok();
        Ok(Single {
            is_single: false,
            pid,
            info: info.to_string(),
            path: p_str,
            _file: None,
        })
    }
}

impl Drop for Single {
    fn drop(&mut self) {
        let option = self._file.take();
        if let Some(_) = option {
            if let Err(e) = fs::remove_file(&self.path) {
                log::error!("remove lock file err! {}", e)
            }
        }
    }
}
