mod http;
mod route_config;
mod route_global;
mod route_kernel;
mod route_rule;
mod route_setting;
mod route_subscribe;
pub mod settings;
mod singbox;
mod startup;
mod tbl_config;
mod tbl_rule;
mod tbl_setting;
mod tbl_subscribe;
pub mod updater;
pub mod webserver;

pub use crate::singbox::start;
pub use crate::singbox::stop;
use library_core::core::AnyResult;

pub fn init() -> AnyResult<()> {
    singbox::init()?;
    Ok(())
}
