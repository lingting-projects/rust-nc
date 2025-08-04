use crate::core::{is_root, restart_root, AnyResult};
use crate::snowflake::next_str;
use crate::{file, sqlite};
use log::LevelFilter;
use simple_logger::SimpleLogger;
use std::{
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
    sync::OnceLock,
};
use time::{format_description::FormatItem, macros::format_description};

#[derive(Debug)]
pub struct Application {
    // 常量属性
    pub id: &'static str,
    pub ua: &'static str,
    pub owner: &'static str,
    pub repo: &'static str,

    // 启动id
    pub start_id: String,

    // 目录路径
    pub global_dir: PathBuf,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub tmp_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub ui_dir: PathBuf,
    /// 运行目录
    pub startup_dir: PathBuf,
    /// 安装目录
    pub install_dir: PathBuf,

    // 配置属性
    pub is_dev: bool,
    pub run_on_root: bool,
}

impl Application {
    pub fn new() -> Self {
        // 常量属性
        let id = "live.lingting.nc.rust";
        let ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/135.0.0.0 Safari/537.36";
        let owner = "lingting-projects";
        let repo = "rust-nc";

        // 生成初始 ID
        let start_id = next_str();

        // 确定全局目录
        let global_dir = {
            let dir = if cfg!(windows) {
                env::var("ALLUSERSPROFILE").expect("ALLUSERSPROFILE environment variable not set")
            } else if cfg!(target_os = "linux") {
                "/usr/local/share/".to_string()
            } else {
                "/Library/Application Support/".to_string()
            };

            let path = PathBuf::from(dir).join(id);

            file::create_dir(&path).expect("Failed to create global directory");
            path
        };

        // 创建其他目录
        let data_dir = global_dir.join("data");
        file::create_dir(&data_dir).expect("Failed to create data_dir");

        let cache_dir = global_dir.join("cache");
        file::create_dir(&cache_dir).expect("Failed to create cache_dir");

        let tmp_dir = {
            let tmp_dir = env::temp_dir();
            let path = tmp_dir.join(id);
            file::create_dir(&path).expect("Failed to create tmp directory");
            path
        };
        let logs_basic_dir = tmp_dir.join("logs");
        file::create_dir(&logs_basic_dir).expect("Failed to create logs_basic_dir");

        let logs_dir = logs_basic_dir.join(&start_id);
        file::create_dir(&logs_dir).expect("Failed to create logs_dir");

        let ui_dir = global_dir.join("ui");
        file::create_dir(&ui_dir).expect("Failed to create ui_dir");

        // 获取启动目录和安装目录
        let startup_dir = env::current_dir().expect("Failed to get current directory");
        log::trace!("运行目录: {}", startup_dir.display());
        let install_dir = {
            env::current_exe()
                .expect("Failed to get executable path")
                .parent()
                .expect("Failed to get parent directory of executable")
                .to_path_buf()
        };
        log::trace!("安装目录: {}", install_dir.display());

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
            id,
            ua,
            owner,
            repo,
            start_id,
            global_dir,
            data_dir,
            cache_dir,
            tmp_dir,
            logs_dir,
            ui_dir,
            startup_dir,
            install_dir,
            is_dev,
            run_on_root,
        }
    }
}

pub static APP: OnceLock<Application> = OnceLock::new();

const TIMESTAMP_FORMAT: &[FormatItem] =
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]");

pub fn init() -> AnyResult<()> {
    let mut level = LevelFilter::Info;
    if cfg!(feature = "trace") {
        level = LevelFilter::Trace;
    } else if cfg!(feature = "debug") {
        level = LevelFilter::Debug;
    }

    let logger = SimpleLogger::new()
        .with_local_timestamps()
        .with_timestamp_format(TIMESTAMP_FORMAT)
        .with_level(level);
    logger.init().unwrap();
    log::info!("完成日志初始化, 日志级别: {level}");
    log::debug!("初始化应用程序基础数据");
    APP.get_or_init(Application::new);
    log::debug!("初始化数据库");
    sqlite::init()?;
    log::debug!("初始化完成");

    Ok(())
}
