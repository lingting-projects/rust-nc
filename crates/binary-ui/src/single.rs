use fs2::FileExt;
use library_core::core::{AnyResult, BizError};
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::{fs, process};

pub struct Single {
    pub is_single: bool,
    pub pid: Option<u32>,
    pub info: String,
    pub path: String,
    pub path_info: String,
    file: Option<File>,
}

fn try_unique<P: AsRef<Path>>(p: P) -> AnyResult<File> {
    let mut options = OpenOptions::new();
    options.write(true).create(true);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options
            .share_mode(0x0)
            .attributes(0)
            .security_qos_flags(0x0)
            .custom_flags(0x0)
            .access_mode(0xC0000000);
    }
    #[cfg(not(target_os = "windows"))]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o444);
    }

    let file = options.open(p)?;
    FileExt::lock_exclusive(&file)?;
    Ok(file)
}

fn write(path: &str, pid: u32, info: &str) -> AnyResult<()> {
    let mut file = File::create(path)?;
    let content = format!("{}\n{}", pid, info);

    // 写入PID和信息
    file.set_len(0)?; // 截断文件
    file.write_all(content.as_bytes())?;
    file.flush()?;
    file.sync_all()?;
    Ok(())
}

impl Single {
    pub fn create<P: AsRef<Path>>(p: P, info: &str) -> AnyResult<Single> {
        let path_lock = p.as_ref();
        if let Some(parent) = path_lock.parent() {
            create_dir_all(parent)?;
        }
        let p_str = path_lock.to_str().expect("get path err").to_string();
        let path_info = format!("{}.info", p_str);

        match try_unique(path_lock) {
            Ok(lock) => {
                let pid = process::id();

                match write(&path_info, pid, info) {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("write pid and info to {} err! {}", path_info, e);
                        return Err(Box::new(BizError::SingleWrite));
                    }
                }

                return Ok(Single {
                    is_single: true,
                    pid: Some(pid),
                    info: info.to_string(),
                    path: p_str,
                    path_info,
                    file: Some(lock),
                });
            }
            Err(e) => {
                log::error!("获取独占锁异常! {}", e)
            }
        }

        let mut options = OpenOptions::new();
        options.read(true);

        let mut file = options.open(&path_info)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        let (pid_str, info) = content.split_once('\n').unwrap_or(("0", &content));
        let pid = pid_str.parse::<u32>().ok();
        Ok(Single {
            is_single: false,
            pid,
            info: info.to_string(),
            path: p_str,
            path_info,
            file: None,
        })
    }
}

impl Drop for Single {
    fn drop(&mut self) {
        let option = self.file.take();
        if let Some(file) = option {
            let _ = FileExt::unlock(&file);
            drop(file);
        }
    }
}
