use std::error::Error;
use std::path::PathBuf;

pub type AnyResult<T> = Result<T, Box<dyn Error>>;

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
