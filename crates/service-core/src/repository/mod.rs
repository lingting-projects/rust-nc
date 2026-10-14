//! 数据库原子操作层：一个文件对应一张表，一个方法对应一条 SQL。

pub(crate) mod app_config_repository;
pub(crate) mod config_repository;
mod mapping;
pub(crate) mod rule_repository;
pub(crate) mod subscribe_repository;
