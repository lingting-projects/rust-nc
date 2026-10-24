use std::path::Path;
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use arc_swap::ArcSwapOption;
use lib_db::DbContext;
use lib_singbox::{Phase, SingBox};
use service_core::holder::settings::SettingsHolder;
use service_core::service::config_service::ConfigService;

use crate::config_file;
use crate::file;
use crate::inner_rule_sync;

/// 等待内核 API 就绪的超时时间。
const READY_TIMEOUT: Duration = Duration::from_secs(15);

/// 全局唯一的内核实例。
static INSTANCE: LazyLock<SingBoxInstance> = LazyLock::new(SingBoxInstance::new);

/// 内核运行状态。
///
/// 取自 `lib-singbox` 的进程阶段，「创建中」与「启动中」都归为启动中，
/// 供界面在启动与停止过程尚未完成时给出准确的反馈。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Status {
    /// 未运行
    #[default]
    Stopped,
    /// 启动中
    Starting,
    /// 运行中
    Running,
    /// 停止中
    Stopping,
}

impl From<Phase> for Status {
    fn from(phase: Phase) -> Self {
        match phase {
            Phase::Stopped => Status::Stopped,
            Phase::Spawn | Phase::Starting => Status::Starting,
            Phase::Running => Status::Running,
            Phase::Stopping => Status::Stopping,
        }
    }
}

/// 内核实例。
///
/// 全局唯一，由服务运行时注入数据库上下文，统一负责内核的启动、停止与状态查询。
/// 内核对象本身按需创建后保存在 [`ArcSwapOption`] 中，供规则集转换等场景复用。
pub struct SingBoxInstance {
    /// 内核对象，尚未创建时为空
    singbox: ArcSwapOption<SingBox>,
    /// 配置 id 与失败原因；进程阶段直接读内核对象，不在这里再存一份
    inner: Mutex<Inner>,
    /// 启停操作的串行锁，避免并发启动或启动未完成时停止
    action: tokio::sync::Mutex<()>,
}

/// 内核实例的内部状态。
#[derive(Default)]
struct Inner {
    /// 服务运行时的数据库上下文，未初始化时为空
    context: Option<Arc<DbContext>>,
    /// 正在运行的配置 id
    config_id: Option<i64>,
    /// 最近一次异常的原因
    reason: Option<String>,
    /// 是否正在执行启停操作
    ///
    /// 启动可能由托盘、自动运行或控制台发起，界面只认这一处标记，
    /// 因此任何入口的启动都会让控制台的启动按钮进入加载态。
    acting: bool,
}
/// 启停进行中标记的守卫。
///
/// 启动与停止有多条提前返回的路径，用守卫保证无论成功、失败还是 panic
/// 都会把标记清掉，界面不会永远停在加载态。
struct Acting<'a>(&'a SingBoxInstance);

impl<'a> Acting<'a> {
    /// 置位标记，存活期间 [`SingBoxInstance::acting`] 为真。
    fn new(instance: &'a SingBoxInstance) -> Self {
        instance.lock().acting = true;
        Self(instance)
    }
}

impl Drop for Acting<'_> {
    fn drop(&mut self) {
        self.0.lock().acting = false;
    }
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

    /// 内核状态。
    ///
    /// 直接读内核对象的进程阶段，不加锁也不做 IO：界面每帧都会调用，
    /// 进程退出由内核对象自己的监控线程发现并更新。
    pub fn status(&self) -> Status {
        match self.singbox.load_full() {
            Some(singbox) => singbox.phase().into(),
            // 内核对象尚未创建，等同于未运行
            None => Status::Stopped,
        }
    }

    /// 正在运行的配置 id，未运行时为 `None`。
    ///
    /// 内核已被监控线程判定退出时同步清掉，避免界面还显示上一轮的配置。
    pub fn config_id(&self) -> Option<i64> {
        let singbox = self.singbox.load_full();
        let running = singbox.is_some_and(|singbox| singbox.phase().is_running());
        let mut inner = self.lock();
        if !running {
            inner.config_id = None;
        }
        inner.config_id
    }

    /// 最近一次异常的原因。
    ///
    /// 启动失败由启动流程记录；运行期间异常退出由内核对象的监控线程记录，
    /// 这里在内核已停止时一并取回，界面据此展示启动/运行失败日志。
    pub fn reason(&self) -> Option<String> {
        let singbox = self.singbox.load_full();
        let mut inner = self.lock();

        if let Some(singbox) = singbox
            && !singbox.phase().is_running()
            && let Some(reason) = singbox.reason()
        {
            inner.reason = Some(reason);
        }
        inner.reason.clone()
    }

    /// 是否正在执行启停操作。
    ///
    /// 启动与停止可能来自控制台、托盘或启动时的自动运行，界面统一读这里，
    /// 因此自动启动期间控制台的启动按钮同样处于加载态。
    pub fn acting(&self) -> bool {
        self.lock().acting
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

    /// 内核 gRPC 客户端，内核未运行时返回 `None`。
    pub fn grpc(&self) -> Result<Option<Arc<lib_singbox::SingBoxGrpc>>> {
        self.singbox()?.grpc()
    }

    /// 读取出站分组，分组内的节点即节点切换的候选项。
    pub async fn groups(&self) -> Result<Vec<lib_singbox::Group>> {
        let Some(grpc) = self.grpc()? else {
            bail!("内核未运行");
        };
        grpc.groups().await
    }

    /// 在分组 `group_tag` 中切换到节点 `outbound_tag`。
    pub async fn select(&self, group_tag: &str, outbound_tag: &str) -> Result<()> {
        let Some(grpc) = self.grpc()? else {
            bail!("内核未运行");
        };
        grpc.select(group_tag, outbound_tag).await
    }

    /// 推送一个节点到测速队列，返回是否成功入队。
    ///
    /// 队列按顺序逐个测速并自行去重，因此这里只负责把客户端交给队列后入队。
    pub fn url_test(&self, outbound_tag: &str) -> Result<bool> {
        let Some(grpc) = self.grpc()? else {
            bail!("内核未运行");
        };
        let queue = self.singbox()?.url_test();
        queue.set_grpc(Some(grpc));
        Ok(queue.push(outbound_tag))
    }

    /// 连接数据。
    ///
    /// 内核的 `SubscribeConnections` 是长连接，订阅由 `lib-singbox` 常驻维护，
    /// 推送结果保存在共享存储里；这里只把存储交给调用方，不重新建流。
    pub fn connections(&self) -> Result<Arc<lib_singbox::ConnectionStore>> {
        // 先取一次客户端, 内核未运行时连接数据已经没有意义
        self.grpc()?;
        Ok(self.singbox()?.connections())
    }

    /// 内核日志。
    ///
    /// 日志由 `lib-singbox` 的常驻线程增量读取日志文件，这里只把存储交给调用方；
    /// 内核未运行时没有可读内容，直接返回空快照。
    pub fn logs(&self) -> Result<Arc<lib_singbox::LogStore>> {
        Ok(self.singbox()?.logs())
    }

    /// 读取一个日志块的完整内容，日志文件已被重写时返回 `None`。
    ///
    /// `progress` 每读完一块调用一次并汇报已读字节，返回 `false` 时立即停止读取。
    pub fn log_detail(
        &self,
        block: &lib_singbox::LogBlock,
        progress: impl FnMut(u64) -> bool,
    ) -> Result<Option<String>> {
        Ok(self.singbox()?.logs().read(block, progress))
    }

    /// 清空内存中缓存的全部连接，之后只保留新推送的数据。
    ///
    /// 只清缓存，不关闭内核里的连接；内核未运行时没有缓存可清，直接返回。
    pub fn clear_connections(&self) -> Result<()> {
        if self.grpc()?.is_none() {
            return Ok(());
        }
        self.singbox()?.connections().clear();
        Ok(())
    }
    /// 关闭指定连接。
    pub async fn close_connection(&self, id: &str) -> Result<()> {
        let Some(grpc) = self.grpc()? else {
            bail!("内核未运行");
        };
        grpc.close_connection(id).await
    }

    /// 启动配置 `id` 的内核。
    ///
    /// 启动命令执行完毕不代表内核可用，因此等内核 API 探测通过后才置为运行中。
    /// 进程本身由内核对象的监控线程创建，这里只登记意图并等待结果。
    /// 启动期间保持「进行中」标记，任意路径结束都会清掉。
    pub async fn start(&self, id: i64) -> Result<()> {
        let _action = self.action.lock().await;
        let _acting = Acting::new(self);
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
        // 配置只引用本地规则集, 规则集未落盘时内核会一直卡在下载、API 不会监听
        // 先补一次登记, 避免配置文件已存在但内置规则未登记的情况
        if let Err(error) = config_file::register_inner_rules(&context, &config).await {
            let reason = format!("{error:#}");
            log::error!("[内核] 启动失败: {reason}");
            self.set(None, Some(reason));
            return Err(error);
        }

        if let Err(error) = inner_rule_sync::wait_ready(&context).await {
            let reason = format!("{error:#}");
            log::error!("[内核] 启动失败: {reason}");
            self.set(None, Some(reason));
            return Err(error);
        }

        let kernel = SettingsHolder::kernel();
        let api = format!("{}:{}", kernel.ui_listen, kernel.ui_port);
        // 内核统一在 singbox_workspace 下启动, 日志与运行时文件都落在这里
        let work = file::sing_box_workspace()?;
        let singbox = self.singbox()?;
        self.set(Some(id), None);
        if let Err(error) = Self::launch(&singbox, &json, &work, &api).await {
            // 启动失败时不留残余进程, 状态回到未运行
            let _ = singbox.stop();
            let reason = format!("{error:#}");
            log::error!("[内核] 启动失败: {reason}");
            self.set(None, Some(reason));
            return Err(error);
        }

        log::info!("[内核] 已启动, 配置: {id}, API: {api}");
        self.set(Some(id), None);
        Ok(())
    }

    /// 让内核重载正在运行的配置, 不重启内核进程。
    ///
    /// 只对内核正在运行的配置生效: `id` 不是运行中的配置或内核未运行时返回 `false`;
    /// 配置文件由调用方负责生成, 这里只通知内核按新配置重建实例。
    pub fn reload(&self, id: i64) -> Result<bool> {
        if self.config_id() != Some(id) {
            return Ok(false);
        }
        if self.status() != Status::Running {
            return Ok(false);
        }

        self.singbox()?.reload(&file::sing_box_workspace()?)?;
        log::info!("[内核] 配置已刷新, 配置: {id}");
        Ok(true)
    }

    /// 停止内核。
    ///
    /// 结束进程由内核对象的监控线程完成，这里只登记停止意图并等待进程回收。
    pub async fn stop(&self) -> Result<()> {
        let _action = self.action.lock().await;
        let _acting = Acting::new(self);
        let Some(singbox) = self.singbox.load_full() else {
            self.set(None, None);
            return Ok(());
        };

        self.set(None, None);
        let result = singbox.stop();
        match &result {
            Ok(()) => {
                log::info!("[内核] 已停止");
                self.set(None, None);
            }
            Err(error) => {
                let reason = format!("{error:#}");
                log::error!("[内核] 停止失败: {reason}");
                self.set(None, Some(reason));
            }
        }
        result
    }

    /// 启动内核进程并等待 API 就绪。
    async fn launch(singbox: &SingBox, json: &Path, work: &Path, api: &str) -> Result<()> {
        singbox.start(json, work, api)?;
        singbox.wait_ready(READY_TIMEOUT).await
    }

    /// 同步正在运行的配置 id 与失败原因。
    ///
    /// 进程阶段由内核对象持有，这里只记录阶段之外的信息。
    fn set(&self, config_id: Option<i64>, reason: Option<String>) {
        let mut inner = self.lock();
        inner.config_id = config_id;
        inner.reason = reason;
    }

    /// 取服务运行时的数据库上下文。
    fn context(&self) -> Result<Arc<DbContext>> {
        self.lock()
            .context
            .clone()
            .ok_or_else(|| anyhow!("内核实例未初始化"))
    }

    /// 取内部状态，锁被污染时继续使用其内容。
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|error| error.into_inner())
    }
}
