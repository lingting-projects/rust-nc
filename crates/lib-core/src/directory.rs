use anyhow::{Context, Result};
use arc_swap::ArcSwapOption;
use framework_core::ApplicationDirectory;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock};

static DIRECTORY: LazyLock<ArcSwapOption<Directorys>> = LazyLock::new(ArcSwapOption::empty);

pub struct Directorys {
    pub data: PathBuf,
    pub cache: PathBuf,
    pub tmp: PathBuf,
    pub logs: PathBuf,
    pub startup: PathBuf,
    pub install: PathBuf,
    // 配置文件根文件夹, 子目录是配置id, 子目录中的才是配置相关的文件
    pub config: PathBuf,
    // 规则文件根文件夹, 子目录是规则id, 子目录中的才是规则相关文件
    pub rule: PathBuf,
    // sing-box 工作目录, 内核在此目录下启动, 日志与缓存等运行时文件都落在这里
    pub singbox_workspace: PathBuf,
}

pub fn directorys() -> Result<Arc<Directorys>> {
    if let Some(v) = DIRECTORY.load_full() {
        return Ok(v.clone());
    }

    let directory = ApplicationDirectory::root("live.lingting.nc.rust")?;
    let root = directory
        .data
        .parent()
        .expect("程序文件夹解析异常!")
        .to_path_buf();
    let v = Arc::new(Directorys {
        data: directory.data,
        cache: directory.cache,
        tmp: directory.tmp,
        logs: directory.logs,
        startup: directory.startup,
        install: directory.install,
        config: create_directory(root.join("config"))?,
        rule: create_directory(root.join("rule"))?,
        singbox_workspace: create_directory(root.join("singbox"))?,
    });
    DIRECTORY.store(Some(v.clone()));
    Ok(v)
}

fn create_directory(directory: PathBuf) -> Result<PathBuf> {
    fs::create_dir_all(&directory)
        .with_context(|| format!("创建应用目录失败: {}", directory.display()))?;
    Ok(directory.to_path_buf())
}
