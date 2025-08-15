mod http;
mod route_config;
mod route_global;
mod route_kernel;
mod route_rule;
mod route_setting;
mod route_subscribe;
pub mod settings;
mod tbl_config;
mod tbl_rule;
mod tbl_setting;
mod tbl_subscribe;
pub mod webserver;
mod singbox;
mod startup;
pub mod updater;

pub use crate::route_kernel::set_start;
