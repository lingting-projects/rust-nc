use crate::window::dispatch;
use library_core::core::{AnyResult, Exit};
use library_web::webserver;
use std::panic;
use std::process::exit;
use tao::dpi::PhysicalSize;
use tao::platform::windows::IconExtWindows;
use tao::window::Icon;

#[derive(Clone, Copy, PartialEq)]
pub enum LoadingState {
    InitSystem,
    InitDb,
    CheckUpdate,
    Updating,
    LoadingAssets,
    Completed,
    UiError,
}

impl LoadingState {
    pub const fn progress(self) -> u8 {
        match self {
            LoadingState::InitSystem => 0,
            LoadingState::InitDb => 25,
            LoadingState::CheckUpdate => 50,
            LoadingState::Updating => 65,
            LoadingState::LoadingAssets => 75,
            LoadingState::Completed => 100,
            LoadingState::UiError => 0,
        }
    }

    pub const fn title(self) -> &'static str {
        match self {
            LoadingState::InitSystem => "正在初始化系统",
            LoadingState::InitDb => "正在初始化数据库",
            LoadingState::CheckUpdate => "正在检查更新",
            LoadingState::Updating => "正在更新",
            LoadingState::LoadingAssets => "正在加载资源",
            LoadingState::Completed => "系统初始化完成",
            LoadingState::UiError => "UI加载异常",
        }
    }

    pub const fn message(self) -> &'static str {
        match self {
            LoadingState::InitSystem => "准备系统环境",
            LoadingState::InitDb => "正在初始化数据库",
            LoadingState::CheckUpdate => "正在检查更新",
            LoadingState::Updating => "正在更新",
            LoadingState::LoadingAssets => "正在加载资源",
            LoadingState::Completed => "正在进入系统",
            LoadingState::UiError => "Ui加载异常, 请尝试重启程序",
        }
    }

    // 获取带前缀的窗口标题
    pub const fn window_title(self) -> &'static str {
        match self {
            LoadingState::InitSystem => "nc-正在初始化系统",
            LoadingState::InitDb => "nc-正在初始化数据库",
            LoadingState::CheckUpdate => "nc-正在检查更新",
            LoadingState::Updating => "nc-正在进行更新",
            LoadingState::LoadingAssets => "nc-正在加载资源",
            LoadingState::Completed => "nc-系统初始化完成",
            LoadingState::UiError => "nc-Ui加载异常",
        }
    }
}

pub const FIRST: LoadingState = LoadingState::InitSystem;

fn emit(state: LoadingState) -> AnyResult<()> {
    log::debug!("[初始化] {}", state.title());

    dispatch(move |w, wv| {
        w.set_title(state.window_title());

        let js_code = format!(
            "window.to({}, '{}', '{}', '{}');",
            state.progress(),
            state.title(),
            state.message(),
            "info"
        );
        match wv.evaluate_script(&js_code) {
            Ok(_) => {}
            Err(e) => {
                log::error!("执行js异常! {}", e);
                exit(Exit::WebViewEvaluateJsError.code())
            }
        }
    })
}

pub fn start_init() {
    match panic::catch_unwind(|| _init()) {
        Ok(r) => match r {
            Ok(_) => {}
            Err(e) => {
                match e.source() {
                    None => {
                        log::error!("初始化异常! 未知异常!");
                    }
                    Some(e) => {
                        log::error!("初始化异常! {}", e);
                    }
                }
                exit(Exit::InitError.code())
            }
        },
        Err(p) => {
            // 处理 panic 信息
            let error_msg = match p.downcast_ref::<String>() {
                Some(s) => s.to_string(),
                None => match p.downcast_ref::<&str>() {
                    Some(s) => (*s).to_string(),
                    None => "panic 未提供错误信息".to_string(),
                },
            };

            log::error!("初始化过程中发生严重错误: {}", error_msg);
            exit(Exit::InitPanicError.code())
        }
    }
}

fn _init() -> AnyResult<()> {
    emit(LoadingState::InitSystem)?;
    init_system()?;
    emit(LoadingState::InitDb)?;
    init_db();
    emit(LoadingState::CheckUpdate)?;
    let option = check_update();
    if option.is_some() {
        emit(LoadingState::Updating)?;
        update(option.unwrap())
    }
    emit(LoadingState::LoadingAssets)?;
    assets();
    emit(LoadingState::Completed)?;
    completed();
    Ok(())
}

fn init_system() -> AnyResult<()> {
    #[cfg(target_os = "windows")]
    let icon = Icon::from_path("icons/256x256.ico", Some(PhysicalSize::new(256, 256)))?;

    #[cfg(not(target_os = "windows"))]
    let icon = Icon::from_path("icons/256x256.png", Some(PhysicalSize::new(256, 256)))?;

    dispatch(move |w, _| {
        w.set_window_icon(Some(icon));
        w.set_visible(true);
    })?;

    webserver::start()?;
    Ok(())
}

fn init_db() {}

fn check_update() -> Option<String> {
    None
}

fn update(url: String) {}

fn assets() {}

fn completed() {
    #[cfg(feature = "local-ui")]
    let app = APP.wait();
    #[cfg(feature = "local-ui")]
    let url = format!("file:///{}", app.ui_dir.to_str().unwrap());
    #[cfg(not(feature = "local-ui"))]
    let url = String::from("http://localhost:30000");

    dispatch(move |_, wv| match wv.load_url(&url) {
        Ok(_) => {}
        Err(_) => emit(LoadingState::UiError).unwrap(),
    })
    .unwrap()
}
