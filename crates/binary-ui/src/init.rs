use crate::icon;
use crate::view::UiView;
use crate::window::dispatch;
use library_core::app::get_app;
use library_core::core::{AnyResult, BizError, Exit};
use library_core::data_size::DataSize;
use library_web::webserver::{WebServer, SERVER};
use library_web::{settings, updater, webserver};
use std::process::exit;
use std::sync::{Mutex, OnceLock};
use std::{panic, thread};
use tao::dpi::PhysicalSize;
use tao::platform::windows::IconExtWindows;
use tao::window::{Icon, Window};
use wry::WebView;

#[derive(Clone, Copy, PartialEq)]
pub enum LoadingState {
    InitSystem,
    InitDb,
    CheckUpdate,
    Updating,
    LoadingAssets,
    Completed,
    UiError,
    ServerError,
}

impl LoadingState {
    pub const fn progress(self) -> u8 {
        match self {
            LoadingState::InitSystem => 5,
            LoadingState::InitDb => 25,
            LoadingState::CheckUpdate => 50,
            LoadingState::Updating => 65,
            LoadingState::LoadingAssets => 75,
            LoadingState::Completed => 100,
            LoadingState::UiError => 0,
            LoadingState::ServerError => 0,
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
            LoadingState::ServerError => "服务启动异常",
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
            LoadingState::ServerError => "服务启动异常, 请尝试重启程序",
        }
    }

    // 获取带前缀的窗口标题
    pub const fn window_title(self) -> &'static str {
        match self {
            LoadingState::InitSystem => "NC-正在初始化系统",
            LoadingState::InitDb => "NC-正在初始化数据库",
            LoadingState::CheckUpdate => "NC-正在检查更新",
            LoadingState::Updating => "NC-正在进行更新",
            LoadingState::LoadingAssets => "NC-正在加载资源",
            LoadingState::Completed => "NC-系统初始化完成",
            LoadingState::UiError => "NC-Ui加载异常",
            LoadingState::ServerError => "NC-服务启动异常",
        }
    }
}

pub const FIRST: LoadingState = LoadingState::InitSystem;

static INIT_ERROR: OnceLock<bool> = OnceLock::new();

fn emit(state: LoadingState) {
    if state.progress() <= 0 {
        INIT_ERROR.get_or_init(|| false);
    }
    log::debug!("[初始化] 提交事件: {}", state.title());
    let closure = move |w: &Window, wv: &dyn UiView| {
        w.set_title(state.window_title());

        let js_code = format!(
            "window.to({}, '{}', '{}', '{}');",
            state.progress(),
            state.title(),
            state.message(),
            "info"
        );
        match wv.eval(&js_code) {
            Ok(_) => {}
            Err(e) => {
                log::error!("执行js异常! {}", e);
                exit(Exit::WebViewEvaluateJsError.code())
            }
        }
    };

    if let Err(e) = dispatch(closure) {
        log::error!("前端加载事件提交异常! state: {}; {}", state.title(), e);
        exit(Exit::UiEmitError.code())
    }
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
    emit(LoadingState::InitSystem);
    init_system()?;
    if INIT_ERROR.get().is_some() {
        return Err(Box::new(BizError::UiInit));
    }
    emit(LoadingState::InitDb);
    init_db();
    if INIT_ERROR.get().is_some() {
        return Err(Box::new(BizError::UiInit));
    }
    emit(LoadingState::CheckUpdate);
    let option = check_update();
    if let Some((version, url, size)) = option {
        emit(LoadingState::Updating);
        update(version, url, size);
        if INIT_ERROR.get().is_some() {
            return Err(Box::new(BizError::UiInit));
        }
    }
    emit(LoadingState::LoadingAssets);
    assets();
    if INIT_ERROR.get().is_some() {
        return Err(Box::new(BizError::UiInit));
    }
    emit(LoadingState::Completed);
    completed();
    if INIT_ERROR.get().is_some() {
        return Err(Box::new(BizError::UiInit));
    }
    Ok(())
}

fn init_system() -> AnyResult<()> {
    let icon = Icon::from_path(
        icon::path,
        Some(PhysicalSize::new(icon::width, icon::height)),
    )?;

    dispatch(move |w, _| {
        if !*settings::is_minimize {
            w.set_visible(true)
        }
        w.set_window_icon(Some(icon));
    })?;

    webserver::wake()?;
    start_web()?;
    Ok(())
}

fn start_web() -> AnyResult<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .build()?;

    let f = runtime.block_on(async {
        match webserver::build().await {
            Ok((port, server)) => {
                let web_server = WebServer {
                    port,
                    runtime: None,
                };
                if let Err(_) = SERVER.set(Mutex::new(web_server)) {
                    log::error!("Web全局缓存设置异常");
                    emit(LoadingState::ServerError);
                    return false;
                }

                tokio::spawn(async {
                    log::debug!("启动服务!");
                    if let Err(e) = server.await {
                        log::error!("服务启动异常! {}", e);
                        emit(LoadingState::ServerError);
                    }
                    log::debug!("服务已关闭!");
                });

                true
            }
            Err(e) => {
                log::error!("服务构建异常! {}", e);
                emit(LoadingState::ServerError);
                false
            }
        }
    });

    // 全局存储 runtime
    if f {
        let x = SERVER.get().unwrap();
        let mut guard = x.lock().unwrap();
        guard.runtime = Some(runtime)
    }

    Ok(())
}

fn init_db() {
    // 留着, 后面说不定需要
}

fn check_update() -> Option<(String, String, DataSize)> {
    updater::check().ok().flatten()
}

fn update(version: String, url: String, size: DataSize) {
    log::debug!("检测到新版本: {}", &version);
    log::debug!("下载地址: {}", &url);
    log::debug!("大小: {}", &size);
}

fn assets() {}

fn completed() {
    let url = if cfg!(not(feature = "local-ui")) {
        String::from("http://localhost:30000")
    } else {
        let app = get_app();
        format!("file:///{}/index.html", app.ui_dir.to_str().unwrap())
    };

    dispatch(move |w, wv| {
        w.set_title("NC");
        match wv.load(&url) {
            Ok(_) => {}
            Err(_) => emit(LoadingState::UiError),
        }
    })
    .unwrap()
}
