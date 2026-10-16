//! 应用数据层。
//!
//! 持有 tokio 运行时与数据库上下文，页面通过它读取配置与订阅。
//! 查询逻辑复用 `service-core` 的服务，本层只负责运行时、连接池与入口。

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context as _, Result};
use gpui_kit::*;
use lib_core::directorys;
use lib_db::{Database, DatabaseOptions, DbContext};
use service_core::manager::setting_manager::SettingManager;
use service_core::service::config_service::ConfigService;
use service_core::service::subscribe_service::SubscribeService;
use types_core::entity::config::Config;
use types_core::entity::subscribe::Subscribe;

/// 一次控制台加载所需的全部数据。
pub struct ConsoleData {
    pub configs: Vec<Config>,
    pub subscribes: Vec<Subscribe>,
    /// 上次使用的配置 id，取自 `setting:run:selected`。
    pub last_used: Option<String>,
}

/// 数据层入口。
pub struct Services {
    runtime: tokio::runtime::Runtime,
    context: Arc<DbContext>,
    cache_dir: PathBuf,
}

impl Services {
    /// 在应用运行时上派生一个查询任务。
    ///
    /// 调用方在 GPUI 任务里 await 返回的句柄即可，查询本身跑在 tokio 线程上。
    pub fn spawn<T>(
        &self,
        future: impl Future<Output = T> + Send + 'static,
    ) -> tokio::task::JoinHandle<T>
    where
        T: Send + 'static,
    {
        self.runtime.spawn(future)
    }

    /// 读取控制台需要的配置、订阅与上次使用的配置 id。
    pub async fn console_data(self: Arc<Self>) -> Result<ConsoleData> {
        let configs = ConfigService::new(self.context.clone()).list().await?;
        let subscribes = SubscribeService::new(self.context.clone()).list().await?;
        let last_used = SettingManager::new(self.context.clone(), self.cache_dir.clone())
            .run()
            .await?
            .selected;

        Ok(ConsoleData {
            configs,
            subscribes,
            last_used,
        })
    }
}

/// 数据层全局。初始化失败时保留原因，页面据此给出提示而不是空白。
pub enum AppServices {
    Ready(Arc<Services>),
    Failed(SharedString),
}

impl Global for AppServices {}

/// 初始化数据层。必须在创建窗口之前调用。
pub fn init(cx: &mut App) {
    match open() {
        Ok(services) => cx.set_global(AppServices::Ready(Arc::new(services))),
        Err(error) => {
            eprintln!("初始化数据层失败: {error:#}");
            cx.set_global(AppServices::Failed(format!("{error:#}").into()));
        }
    }
}

/// 读取数据层。未初始化时返回 `None`。
pub fn get(cx: &App) -> Option<&AppServices> {
    cx.try_global::<AppServices>()
}

/// 打开数据库并建好运行时。
fn open() -> Result<Services> {
    let directory = directorys().context("获取应用目录失败")?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("创建异步运行时失败")?;
    let database = runtime
        .block_on(Database::connect(DatabaseOptions::new(
            directory.data.clone(),
        )))
        .context("打开数据库失败")?;

    Ok(Services {
        runtime,
        context: Arc::new(DbContext::new(database.pool().clone())),
        cache_dir: directory.cache.clone(),
    })
}
