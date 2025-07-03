use std::error::Error;

pub type AnyResult<T> = Result<T, Box<dyn Error>>;
