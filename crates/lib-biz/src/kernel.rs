use std::path::Path;
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use arc_swap::ArcSwapOption;
use lib_core::directorys;
use lib_db::DbContext;
use lib_singbox::SingBox;
use service_core::manager::setting_manager::SettingManager;
use service_core::service::config_service::ConfigService;
use types_core::dto::setting::SettingKernel;

use crate::config_file;
use crate::file;

/// 等待内核 API 就绪的超时时间。
const READY_TIMEOUT: Duration = Duration::from_secs(15);

/// 全局唯一的内核实例。
static INSTANCE: LazyLock<SingBoxInstance> = LazyLock::new(SingBoxInstance::new);

/// 内核运行状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Status {
    /// 未运行
    #[default]
    Stopped,
    /// 启动中
    Starting,
    /// 运行中
    Running,
}

/// 内核实例。
///
/// 全局唯一，由服务运行时注入数据库上下文，统一负责内核的启动、停止与状态查询。
/// 内核对象本身按需创建后保存在 [`ArcSwapOption`] 中，供规则集转换等场景复用。
pub struct SingBoxInstance {
    /// 内核对象，尚未创建时为空
    singbox: ArcSwapOption<SingBox>,
    inner: Mutex<Inner>,
    /// 启停操作的串行锁，避免并发启动或启动未完成时停止
    action: tokio::sync::Mutex<()>,
}

/// 内核实例的内部状态。
#[derive(Default)]
struct Inner {
    /// 服务运行时的数据库上下文，未初始化时为空
    context: Option<Arc<DbContext>>,
    status: Status,
    /// 正在运行的配置 id
    config_id: Option<String>,
    /// 最近一次异常的原因
    reason: Option<String>,
}

impl SingBoxInstance {
    /// 创建实例，仅由全局静态量调用。
    fn new() -> Self {
        Self {
            singbox: ArcSwapOption::empty(),
            inner: Mutex::new(Inner::default()),
            action: tokio::sync::Mutex::new(()),
        }
    }

    /// 全局唯一实例。
    pub fn instance() -> &'static Self {
        &INSTANCE
    }

    /// 注入服务运行时的数据库上下文，重复调用会覆盖旧值。
    pub fn init(&self, context: Arc<DbContext>) {
        self.lock().context = Some(context);
    }

    /// 内核状态，读取前先同步进程的真实状态。
    pub fn status(&self) -> Status {
        if let Err(error) = self.refresh() {
            log::warn!("[内核] 刷新状态失败: {error:#}");
        }
        self.lock().status
    }

    /// 正在运行的配置 id，未运行时为 `None`。
    pub fn config_id(&self) -> Option<String> {
        self.lock().config_id.clone()
    }

    /// 最近一次异常的原因。
    pub fn reason(&self) -> Option<String> {
        self.lock().reason.clone()
    }

    /// 内核对象，尚未创建时按安装目录创建并保存。
    pub fn singbox(&self) -> Result<Arc<SingBox>> {
        if let Some(singbox) = self.singbox.load_full() {
            return Ok(singbox);
        }

        let mut inner = self.lock();
        // 持锁后复查, 避免并发创建出两个内核对象
        if let Some(singbox) = self.singbox.load_full() {
            return Ok(singbox);
        }

        let singbox = Arc::new(lib_singbox::create()?);
        self.singbox.store(Some(singbox.clone()));
        inner.reason = None;
        Ok(singbox)
    }

    /// 启动配置 `id` 的内核。
    ///
    /// 启动命令执行完毕不代表内核可用，因此等内核 API 探测通过后才置为运行中。
    pub async fn start(&self, id: &str) -> Result<()> {
        let _action = self.action.lock().await;
        if self.status() != Status::Stopped {
            bail!("内核已在运行或启动中");
        }

        let context = self.context()?;
        let Some(config) = ConfigService::new(context.clone()).find(id).await? else {
            bail!("配置不存在: {id}");
        };

        // 配置文件尚未生成时先生成, 保证内核使用与当前数据一致的配置
        let json = file::sing_box_json(id)?;
        if !json.exists() {
            config_file::write(&context, &config).await?;
        }

        let kernel = self.setting(&context).await?;
        let api = format!("{}:{}", kernel.ui_listen, kernel.ui_port);
        // 内核统一在 singbox_workspace 下启动, 日志与运行时文件都落在这里
        let work = file::sing_box_workspace()?;
        let singbox = self.singbox()?;

        self.set(Status::Starting, Some(id), None);
        if let Err(error) = Self::launch(&singbox, &json, &work, &api).await {
            // 启动失败时不留残余进程, 状态回到未运行
            let _ = singbox.stop();
            let reason = format!("{error:#}");
            log::error!("[内核] 启动失败: {reason}");
            self.set(Status::Stopped, None, Some(reason));
            return Err(error);
        }

        log::info!("[内核] 已启动, 配置: {id}, API: {api}");
        self.set(Status::Running, Some(id), None);
        Ok(())
    }

    /// 停止内核。
    pub async fn stop(&self) -> Result<()> {
        let _action = self.action.lock().await;
        let Some(singbox) = self.singbox.load_full() else {
            self.set(Status::Stopped, None, None);
            return Ok(());
        };

        let result = singbox.stop();
        match &result {
            Ok(()) => {
                log::info!("[内核] 已停止");
                self.set(Status::Stopped, None, None);
            }
            Err(error) => {
                let reason = format!("{error:#}");
                log::error!("[内核] 停止失败: {reason}");
                self.set(Status::Stopped, None, Some(reason));
            }
        }
        result
    }

    /// 启动内核进程并等待 API 就绪。
    async fn launch(singbox: &SingBox, json: &Path, work: &Path, api: &str) -> Result<()> {
        singbox.start(json, work, api)?;
        singbox.wait_ready(READY_TIMEOUT).await
    }

    /// 同步进程的真实状态，进程已退出时回到未运行。
    fn refresh(&self) -> Result<()> {
        let Some(singbox) = self.singbox.load_full() else {
            return Ok(());
        };
        let state = singbox.state()?;

        let mut inner = self.lock();
        if inner.status == Status::Running && !state.running {
            inner.status = Status::Stopped;
            inner.config_id = None;
            inner.reason = state.reason;
        }
        Ok(())
    }

    /// 更新状态与关联信息。
    fn set(&self, status: Status, config_id: Option<&str>, reason: Option<String>) {
        let mut inner = self.lock();
        inner.status = status;
        inner.config_id = config_id.map(str::to_string);
        inner.reason = reason;
    }

    /// 取服务运行时的数据库上下文。
    fn context(&self) -> Result<Arc<DbContext>> {
        self.lock()
            .context
            .clone()
            .ok_or_else(|| anyhow!("内核实例未初始化"))
    }

    /// 读取内核设置。
    async fn setting(&self, context: &Arc<DbContext>) -> Result<SettingKernel> {
        SettingManager::new(context.clone(), directorys()?.cache.clone())
            .kernel()
            .await
    }

    /// 取内部状态，锁被污染时继续使用其内容。
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|error| error.into_inner())
    }
}
