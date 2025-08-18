use crate::core::{is_root, panic_msg, restart_root, AnyResult};
use crate::snowflake::next_str;
use crate::{file, sqlite};
use std::ops::Deref;
use std::sync::LazyLock;
use std::{
    env,
    error::Error,
    fs, panic,
    path::{Path, PathBuf},
    sync::OnceLock,
};

static ID: &'static str = "live.lingting.nc.rust";
static UA: &'static str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/135.0.0.0 Safari/537.36";
static OWNER: &'static str = "lingting-projects";
static REPO: &'static str = "rust-nc";

static START_ID: LazyLock<String> = LazyLock::new(|| next_str());

static DIR_INSTALL: LazyLock<PathBuf> = LazyLock::new(|| {
    env::current_exe()
        .expect("Failed to get executable path")
        .parent()
        .expect("Failed to get parent directory of executable")
        .to_path_buf()
});
static DIR_GLOBAL: LazyLock<PathBuf> = LazyLock::new(|| {
    let dir = if (cfg!(debug_assertions)) {
        DIR_INSTALL
            .join("runtime")
            .to_str()
            .expect("failed get debug runtime dir")
            .to_string()
    } else if cfg!(windows) {
        env::var("ALLUSERSPROFILE").expect("ALLUSERSPROFILE environment variable not set")
    } else if cfg!(target_os = "linux") {
        "/usr/local/share/".to_string()
    } else {
        "/Library/Application Support/".to_string()
    };

    let mut path = PathBuf::from(dir);
    if (cfg!(not(debug_assertions))) {
        path = path.join(ID)
    }

    file::create_dir(&path).expect("Failed to create global directory");
    path
});
static DIR_TEMP: LazyLock<PathBuf> = LazyLock::new(|| {
    if cfg!(debug_assertions) {
        return DIR_GLOBAL.join("temp");
    }
    let tmp_dir = env::temp_dir();
    let path = tmp_dir.join(ID);
    file::create_dir(&path).expect("Failed to create tmp directory");
    path
});
static DIR_LOGS: LazyLock<PathBuf> = LazyLock::new(|| {
    let dir = DIR_TEMP.join("logs").join(&*START_ID);
    file::create_dir(&dir).expect("Failed to create logs dir");
    dir
});

static DIR_DATA: LazyLock<PathBuf> = LazyLock::new(|| {
    let dir = DIR_GLOBAL.join("data");
    file::create_dir(&dir).expect("Failed to create da a_dir");
    dir
});
static DIR_CACHE: LazyLock<PathBuf> = LazyLock::new(|| {
    let dir = DIR_GLOBAL.join("cache");
    file::create_dir(&dir).expect("Failed to create ca he_dir");
    dir
});
static DIR_UI: LazyLock<PathBuf> = LazyLock::new(|| {
    let dir = DIR_GLOBAL.join("ui");
    file::create_dir(&dir).expect("Failed to create ui dir");
    dir
});

#[derive(Debug)]
pub struct Application {
    // 常量属性
    pub id: &'static str,
    pub ua: &'static str,
    pub owner: &'static str,
    pub repo: &'static str,

    // 启动id
    pub start_id: &'static str,

    // 目录路径
    pub global_dir: &'static PathBuf,
    pub data_dir: &'static PathBuf,
    pub cache_dir: &'static PathBuf,
    pub tmp_dir: &'static PathBuf,
    pub logs_dir: &'static PathBuf,
    pub ui_dir: &'static PathBuf,
    /// 运行目录
    pub startup_dir: PathBuf,
    /// 安装目录
    pub install_dir: &'static PathBuf,

    // 配置属性
    pub is_dev: bool,
    pub run_on_root: bool,
}

impl Application {
    pub fn new() -> Self {
        log::trace!("安装目录: {}", DIR_INSTALL.display());
        let startup_dir = env::current_dir().expect("Failed to get current directory");
        log::trace!("运行目录: {}", startup_dir.display());

        // 计算属性
        let mut is_dev = true;
        if cfg!(feature = "prod") {
            is_dev = false
        }

        #[cfg(not(debug_assertions))]
        let mut run_on_root = is_root();
        #[cfg(debug_assertions)]
        let mut run_on_root = true;
        if !run_on_root {
            restart_root();
            run_on_root = true
        }

        Self {
            id: ID,
            ua: UA,
            owner: OWNER,
            repo: REPO,
            start_id: &*START_ID,
            global_dir: &*DIR_GLOBAL,
            data_dir: &*DIR_DATA,
            cache_dir: &*DIR_CACHE,
            tmp_dir: &*DIR_TEMP,
            logs_dir: &*DIR_LOGS,
            ui_dir: &*DIR_UI,
            startup_dir,
            install_dir: &*DIR_INSTALL,
            is_dev,
            run_on_root,
        }
    }
}

static APP: OnceLock<Application> = OnceLock::new();

pub fn get_app() -> &'static Application {
    APP.get().expect("failed get app")
}

pub fn app_wait() -> &'static Application {
    APP.wait()
}

pub fn app_map<R, F: Fn(&'static Application) -> R>(f: F) -> Option<R> {
    APP.get().map(f)
}

#[cfg(feature = "simple_logger")]
fn log_simple() {
    use log::LevelFilter;
    let mut level = LevelFilter::Info;
    if cfg!(feature = "trace") {
        level = LevelFilter::Trace;
    } else if cfg!(feature = "debug") {
        level = LevelFilter::Debug;
    }

    use time::{format_description::FormatItem, macros::format_description};
    const TIMESTAMP_FORMAT: &[FormatItem] =
        format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]");

    use simple_logger::{init, SimpleLogger};
    let logger = SimpleLogger::new()
        .with_local_timestamps()
        .with_timestamp_format(TIMESTAMP_FORMAT)
        .with_level(level);
    logger.init().unwrap();
    log::info!("完成日志初始化, 日志级别: {level}");
}

fn _init() -> AnyResult<()> {
    #[cfg(all(feature = "redirect", not(debug_assertions)))]
    crate::redirect::redirect_dir(&*DIR_LOGS)?;
    #[cfg(feature = "simple_logger")]
    log_simple();

    log::debug!("初始化应用程序基础数据");
    APP.get_or_init(Application::new);
    log::debug!("初始化数据库");
    sqlite::init()?;
    log::debug!("初始化完成");
    Ok(())
}

pub fn init() -> AnyResult<()> {
    match panic::catch_unwind(|| _init()) {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => {
            log::error!("初始化异常! {}", e);
            return Err(e);
        }
        Err(p) => {
            let msg = panic_msg(p);
            log::error!("初始化崩溃! {}", msg);
            panic!("初始化崩溃: {}", msg)
        }
    }
    Ok(())
}
