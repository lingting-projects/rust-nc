use std::error::Error;

pub type AnyResult<T> = Result<T, Box<dyn Error>>;

#[derive(thiserror::Error, Debug)]
pub enum BizError {
    #[error("应用初始化异常")]
    AppConfig,
}
