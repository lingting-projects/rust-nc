//! 数据库与基础业务操作：repository 为单表原子操作，service 为单表业务，manager 为跨表编排。

pub mod manager;
pub(crate) mod repository;
pub mod service;
