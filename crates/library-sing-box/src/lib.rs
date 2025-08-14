#[cfg(feature = "bin")]
mod _bin;
#[cfg(not(feature = "bin"))]
mod _lib;

use library_core::core::AnyResult;
use std::path::Path;

pub static version: &'static str = "v1.11.9";

pub struct State {
    pub running: bool,
    pub error: bool,
    pub reason: Option<String>,
}

pub trait SingBox {
    fn state(&mut self) -> AnyResult<State>;
    fn start(&mut self, config_path: &Path, work_dir: &Path) -> AnyResult<()>;
    fn stop(&mut self) -> AnyResult<()>;
    fn json_srs(&self, json_path: &Path, srs_path: &Path) -> AnyResult<()>;
}

pub fn create() -> AnyResult<Box<dyn SingBox + Send + Sync>> {
    #[cfg(feature = "bin")]
    let i = _bin::new()?;
    #[cfg(not(feature = "bin"))]
    let i = _lib::new()?;
    Ok(Box::new(i))
}
