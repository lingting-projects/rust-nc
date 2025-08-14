use std::any::Any;
use std::error::Error;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::{env, process, thread};

pub type AnyResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[derive(thiserror::Error, Debug)]
pub enum BizError {
    #[error("Web绑定端口异常")]
    WebUnbind,
    #[error("Web启动异常")]
    WebUnstart,
    #[error("Web全局变量设置异常")]
    WebUnset,
    #[error("Ui初始化异常")]
    UiInit,
    #[error("数据库初始化异常")]
    SqliteInit,
    #[error("雪花算法初始化异常! {0}")]
    SnowflakeInit(String),
    #[error("未找到订阅")]
    SubscribeNotFound,
    #[error("未找到规则")]
    RuleNotFound,
    #[error("未找到配置")]
    ConfigNotFound,
    #[error("未找到文件! {0}")]
    PathNotFound(PathBuf),
    #[error("未找到文件! {0}")]
    FileNotFound(String),
    #[error("没有可用节点! {0}")]
    NodesEmpty(String),
    #[error("无效的UTF-8字符串")]
    InvalidUtf8(),
    #[error("字符串转换错误")]
    NulError(#[from] std::ffi::NulError),
    #[error("操作失败，错误码: {0}")]
    OperationFailed(i32),
    #[error("路径不能为空")]
    EmptyPath,
    #[error("不能重复运行")]
    SingleRunning,
    #[error("单进程信息写入异常")]
    SingleWrite,
    #[error("SingBox初始化异常")]
    SingBoxInit,
    #[error("执行超时")]
    Timeout,
    #[error("自启动操作异常")]
    StartupOperation,
    #[error("字符集识别异常")]
    CharsetReadErr,
}

pub enum Exit {
    LoopProxyError,
    WebViewSenderError,
    WebViewEvaluateJsError,
    InitError,
    InitPanicError,
    WebServerError,
    WebServerPanicError,
    UiEmitError,
}

impl Exit {
    pub const fn code(self) -> i32 {
        match self {
            Exit::LoopProxyError => 1,
            Exit::WebViewSenderError => 2,
            Exit::WebViewEvaluateJsError => 3,
            Exit::InitError => 4,
            Exit::InitPanicError => 5,
            Exit::WebServerError => 6,
            Exit::WebServerPanicError => 7,
            Exit::UiEmitError => 8,
        }
    }
}

pub fn panic_msg(p: Box<dyn Any + Send>) -> String {
    match p.downcast_ref::<String>() {
        Some(s) => s.to_string(),
        None => match p.downcast_ref::<&str>() {
            Some(s) => (*s).to_string(),
            None => "panic 未提供错误信息".to_string(),
        },
    }
}

#[cfg(target_os = "windows")]
pub fn is_root() -> bool {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        // 捕获线程内的恐慌
        let result = std::panic::catch_unwind(|| {
            // 构建命令
            let mut cmd = Command::new("cmd");
            cmd.arg("/c")
                .arg("net session")
                .stdout(Stdio::null())
                .stderr(Stdio::null());

            // 执行命令并返回状态
            match cmd.status() {
                Ok(status) => status.success(),
                Err(_) => false,
            }
        });

        // 发送结果（无论是否恐慌）
        let _ = tx.send(result);
    });

    // 等待结果，设置1秒超时
    match rx.recv_timeout(Duration::from_secs(1)) {
        // 成功接收线程返回值
        Ok(Ok(v)) => v,
        // 线程内发生恐慌
        Ok(Err(_)) => false,
        // 超时
        Err(mpsc::RecvTimeoutError::Timeout) => false,
        // 接收通道错误
        Err(mpsc::RecvTimeoutError::Disconnected) => false,
    }
}

#[cfg(not(target_os = "windows"))]
pub fn is_root() -> bool {
    unsafe { libc::getuid() == 0 }
}

#[cfg(target_os = "windows")]
fn start_root(path: &str, args: Vec<String>) {
    let args_str = args.join(" ");
    let ps_command = format!(
        "Start-Process -FilePath \"{}\" -ArgumentList \"{}\" -Verb RunAs",
        path, args_str
    );
    let _ = Command::new("powershell")
        .arg("-Command")
        .arg(&ps_command)
        .status();
}

#[cfg(not(target_os = "windows"))]
fn start_root(path: &str, args: Vec<String>) {
    let _ = Command::new("sudo").arg(path).args(&args[1..]).status();
}

pub fn restart_root() {
    let path = env::current_exe()
        .expect("get current bin path failed")
        .to_str()
        .expect("convert current bin path failed")
        .to_string();
    let args: Vec<String> = env::args().collect();
    start_root(&path, args);
    process::exit(-1)
}

pub fn require_root() {
    if is_root() {
        return;
    }
    restart_root()
}

pub fn current_millis() -> AnyResult<u128> {
    let millis = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    Ok(millis)
}
