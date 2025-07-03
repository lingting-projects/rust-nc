use crate::webview;
use std::error::Error;
use std::process::exit;
use web_view::Handle;

#[derive(Clone, Copy, PartialEq)]
pub enum LoadingState {
    InitSystem,
    InitDb,
    CheckUpdate,
    Updating,
    LoadingAssets,
    Completed,
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
        }
    }
}

pub const FIRST: LoadingState = LoadingState::InitSystem;

fn emit(state: LoadingState) -> Result<(), Box<dyn Error>> {
    log::debug!("[初始化] {}", state.title());
    let handle = webview::handle()?;

    match handle.dispatch(move |wv| match wv.set_title(state.window_title()) {
        Ok(_) => {
            let js_code = format!(
                "window.to({}, '{}', '{}', '{}');",
                state.progress(),
                state.title(),
                state.message(),
                "info"
            );
            wv.eval(&js_code)
        }
        Err(e) => {
            log::error!("修改标题时异常! {}", state.window_title());
            Err(e)
        }
    }) {
        Ok(_) => Ok(()),
        Err(e) => {
            log::error!("提交状态异常! {}; {}", state.title(), e);
            exit(2)
        }
    }
}

pub fn start_init() -> Result<(), Box<dyn Error>> {
    emit(LoadingState::InitSystem)?;
    init_system();
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

fn init_system() {
    
}
fn init_db() {}
fn check_update() -> Option<String> {
    None
}
fn update(url: String) {}
fn assets() {}
fn completed() {}
