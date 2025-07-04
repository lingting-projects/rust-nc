use log::LevelFilter;
use std::{
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
    sync::OnceLock,
};

#[derive(Debug)]
pub struct ApplicationInner {
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
    pub startup_dir: PathBuf,
    pub install_dir: PathBuf,

    // 配置属性
    pub is_dev: bool,
    pub log_level: LevelFilter,
    pub run_on_root: bool,
    pub minimize_file: PathBuf,
    pub run_on_minimize: bool,
}

impl ApplicationInner {
    // 公开构造函数
    pub fn new() -> Self {
        // 常量属性
        let id = "live.lingting.kmp.nc";
        let ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/135.0.0.0 Safari/537.36";
        let owner = "lingting-projects";
        let repo = "lingting-nc";

        // 生成初始 ID
        let start_id = "0".to_string();

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
            create_dir_all(&path).expect("Failed to create global directory");
            path
        };

        // 创建其他目录
        let data_dir = create_sub_dir(&global_dir, "data");
        let cache_dir = create_sub_dir(&global_dir, "cache");
        let tmp_dir = {
            let tmp_dir = env::temp_dir();
            let path = tmp_dir.join(id);
            create_dir_all(&path).expect("Failed to create tmp directory");
            path
        };
        let logs_basic_dir = create_sub_dir(&tmp_dir, "logs");
        let logs_dir = create_sub_dir(&logs_basic_dir, &start_id);
        let ui_dir = create_sub_dir(&global_dir, "ui");

        // 获取启动目录和安装目录
        let startup_dir = env::current_dir().expect("Failed to get current directory");

        let install_dir = {
            let exe_path = env::current_exe().expect("Failed to get executable path");
            let parent = exe_path
                .parent()
                .expect("Failed to get parent directory of executable")
                .parent()
                .expect("Failed to get grandparent directory of executable");
            parent.to_path_buf()
        };

        // 计算属性
        let is_dev = false;

        let log_level = if is_dev {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        };

        let run_on_root = false;

        // 最小化文件路径
        let minimize_file = cache_dir.join("minimize");

        let run_on_minimize = if is_dev {
            false
        } else {
            minimize_file.exists()
        };

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
            log_level,
            run_on_root,
            minimize_file,
            run_on_minimize,
        }
    }
}

// 辅助函数
fn create_dir_all(path: &Path) -> Result<(), Box<dyn Error>> {
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

fn create_sub_dir(parent: &PathBuf, name: &str) -> PathBuf {
    let path = parent.join(name);
    create_dir_all(&path).expect(&format!("Failed to create directory: {}", path.display()));
    path
}

pub static Application: OnceLock<ApplicationInner> = OnceLock::new();

use simple_logger::SimpleLogger;
use time::{format_description::FormatItem, macros::format_description};

const TIMESTAMP_FORMAT: &[FormatItem] =
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]");

pub fn init() {
    Application.get_or_init(ApplicationInner::new);

    let logger = SimpleLogger::new()
        .with_local_timestamps()
        .with_timestamp_format(TIMESTAMP_FORMAT)
        .with_level(LevelFilter::Debug);
    logger.init().unwrap();
}
