use std::error::Error;

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
