//! 内核进程管理。
//!
//! 进程的创建、结束与退出检测全部由一条常驻的监控线程完成，界面线程只读
//! [`Phase`] 与失败原因，两者都存在 `ArcSwap` 里，读取不加锁。
//!
//! 这样拆的原因：创建 36MB 的子进程在首次启动时可能耗时一秒以上，结束进程同样要等
//! 系统回收；这些慢 IO 若发生在持锁区间内，界面每帧调用的状态查询就会被一起阻塞。

use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use arc_swap::{ArcSwap, ArcSwapOption};
use lib_system::process::Process;

#[cfg(feature = "grpc")]
use std::time::Instant;

mod connection;
#[cfg(feature = "grpc")]
mod grpc;
mod logs;
mod url_test;

pub use connection::{ConnectionInfo, ConnectionRecord, ConnectionSnapshot, ConnectionStore};
#[cfg(feature = "grpc")]
pub use grpc::{Group, GroupItem, ProcessInfo, SingBoxGrpc};
pub use logs::{Level as LogLevel, LogBlock, LogSnapshot, LogStore};
pub use url_test::UrlTestQueue;

/// 随包分发的 sing-box 版本。
pub const VERSION: &str = "v1.14.3";

/// 内核进程可执行文件名，位于安装目录下。
pub const BIN_NAME: &str = "lingting-nc-singbox";

/// 内核异常时附加到失败原因中的日志行数。
const TAIL_LINES: usize = 10;

/// 监控线程检查进程退出的间隔。
const POLL_INTERVAL: Duration = Duration::from_secs(1);

/// 等待阶段变化的最长时间。
///
/// 监控线程正常时阶段变化是瞬时的；设上限只为避免监控线程异常退出时调用方无限挂起。
const WAIT_TIMEOUT: Duration = Duration::from_secs(10);

/// 内核生命周期阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Phase {
    /// 未运行。
    #[default]
    Stopped,
    /// 启动中：已登记启动意图，进程尚未创建完成。
    Starting,
    /// 运行中。
    Running,
    /// 停止中：已登记停止意图，进程尚未回收。
    Stopping,
}

impl Phase {
    /// 是否处于运行中。
    pub fn is_running(self) -> bool {
        self == Phase::Running
    }
}

/// 内核实例。
///
/// 仅支持随包分发的二进制内核，实例创建时即绑定可执行文件路径，
/// 之后通过 [`SingBox::start`] 与 [`SingBox::stop`] 管理其生命周期。
/// 实例可由多个线程共享，读取状态不加锁。
pub struct SingBox {
    /// 内核可执行文件路径，创建后不变。
    bin: String,
    /// 与监控线程共享的运行状态。
    shared: Arc<Shared>,
}

/// 与监控线程共享的运行状态。
///
/// 阶段与失败原因用 `ArcSwap` 存放，任何线程都能无锁读取；
/// 启停意图与进程句柄只由 [`SingBox`] 的启停方法和监控线程访问。
struct Shared {
    /// 当前生命周期阶段。
    phase: ArcSwap<Phase>,
    /// 最近一次异常的原因，正常运行期间为空。
    reason: ArcSwapOption<String>,
    /// 内核 API 监听地址，运行期间有效。
    api: ArcSwap<String>,
    /// 启停意图与进程句柄。
    control: Mutex<Control>,
    /// 阶段变化与意图登记的唤醒信号。
    wake: Condvar,
    /// gRPC 客户端，仅在内核运行期间存在。
    grpc: ArcSwapOption<SingBoxGrpc>,
    /// 测速任务队列，随内核对象一起创建，内核启停时注入或清空客户端。
    url_test: Arc<UrlTestQueue>,
    /// 连接数据，内核运行期间由常驻订阅把推送累积成快照。
    connections: Arc<ConnectionStore>,
    /// 内核日志，内核运行期间由常驻读取线程把日志文件解析成块。
    logs: Arc<LogStore>,
}

/// 启停意图与进程句柄，只由启停方法与监控线程访问。
#[derive(Default)]
struct Control {
    /// 尚未处理的意图。
    intent: Intent,
    /// 当前子进程，仅监控线程持有。
    process: Option<Process>,
    /// 当前子进程的工作目录，进程异常退出时据此读取日志尾部的原始错误。
    work: Option<PathBuf>,
    /// 实例是否已释放，监控线程看到后退出。
    closed: bool,
}

/// 启停意图。
#[derive(Default)]
enum Intent {
    /// 无待处理意图。
    #[default]
    Idle,
    /// 启动内核。
    Start {
        config: PathBuf,
        work: PathBuf,
        api: String,
    },
    /// 停止内核。
    Stop,
}

impl Shared {
    /// 创建初始状态：未运行，没有失败原因。
    fn new() -> Self {
        Self {
            phase: ArcSwap::from_pointee(Phase::Stopped),
            reason: ArcSwapOption::empty(),
            api: ArcSwap::from_pointee(String::new()),
            control: Mutex::new(Control::default()),
            wake: Condvar::new(),
            grpc: ArcSwapOption::empty(),
            url_test: Arc::new(UrlTestQueue::new()),
            connections: Arc::new(ConnectionStore::new()),
            logs: Arc::new(LogStore::new()),
        }
    }

    /// 当前阶段，无锁读取。
    fn phase(&self) -> Phase {
        **self.phase.load()
    }

    /// 取内部状态，锁被污染时继续使用其内容。
    fn lock(&self) -> MutexGuard<'_, Control> {
        self.control
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }

    /// 更新阶段与失败原因，并唤醒等待阶段变化的调用方。
    ///
    /// 持锁写入是为了与 [`wait_phase`](Self::wait_phase) 互斥：等待方在锁内检查阶段，
    /// 写入方也在锁内改阶段并唤醒，这样不会出现「等待方刚检查完、唤醒信号已发出」的丢失唤醒。
    fn set_phase(&self, phase: Phase, reason: Option<String>) {
        let _guard = self.lock();
        self.phase.store(Arc::new(phase));
        self.reason.store(reason.map(Arc::new));
        self.wake.notify_all();
    }

    /// 等待阶段满足 `accept`，返回满足条件时的阶段。
    ///
    /// 超时说明监控线程没有响应，返回 `None` 让调用方给出明确错误而不是一直挂起。
    fn wait_phase(&self, accept: impl Fn(Phase) -> bool) -> Option<Phase> {
        let guard = self.lock();
        let (guard, _) = self
            .wake
            .wait_timeout_while(guard, WAIT_TIMEOUT, |_| !accept(self.phase()))
            .unwrap_or_else(|error| error.into_inner());
        drop(guard);

        let phase = self.phase();
        accept(phase).then_some(phase)
    }

    /// 作废与进程绑定的数据：客户端、连接订阅都随旧进程失效。
    fn invalidate(&self) {
        self.grpc.store(None);
        self.url_test.set_grpc(None);
        self.url_test.clear();
        self.connections.subscribe(None);
    }
}

impl SingBox {
    /// 创建实例，`bin_dir` 为内核可执行文件所在目录。
    ///
    /// 同时启动常驻的监控线程，由它负责创建进程、结束进程与检测退出。
    pub fn new(bin_dir: impl AsRef<Path>) -> Result<Self> {
        let bin = bin_dir.as_ref().join(BIN_NAME);
        let bin = bin
            .to_str()
            .with_context(|| format!("内核路径非 UTF-8: {}", bin.display()))?
            .to_string();
        log::info!("load sing box bin from {bin}");

        let shared = Arc::new(Shared::new());
        let watcher = shared.clone();
        let command = bin.clone();
        std::thread::Builder::new()
            .name("singbox-watch".to_string())
            .spawn(move || watch(watcher, command))
            .context("启动内核监控线程失败")?;

        Ok(Self { bin, shared })
    }

    /// 当前生命周期阶段。
    ///
    /// 无锁读取，界面每帧调用也不会阻塞。
    pub fn phase(&self) -> Phase {
        self.shared.phase()
    }

    /// 最近一次异常的原因，正常运行期间为 `None`。
    pub fn reason(&self) -> Option<String> {
        self.shared
            .reason
            .load_full()
            .map(|reason| reason.to_string())
    }

    /// 登记启动意图并唤醒监控线程，立即返回。
    ///
    /// 进程由监控线程创建，因此返回时内核尚未真正启动；
    /// 需要等待启动结果时用 [`start`](Self::start)，需要等 API 就绪时用
    /// [`wait_ready`](Self::wait_ready)。
    pub fn start_async(&self, config_path: &Path, work_dir: &Path, api: &str) -> Result<()> {
        match self.phase() {
            // 已在启动或已运行，重复登记没有意义
            Phase::Starting | Phase::Running => return Ok(()),
            Phase::Stopping => bail!("内核正在停止"),
            Phase::Stopped => {}
        }

        {
            let mut control = self.shared.lock();
            control.intent = Intent::Start {
                config: config_path.to_path_buf(),
                work: work_dir.to_path_buf(),
                api: api.to_string(),
            };
        }
        // 先登记意图再置阶段：监控线程被唤醒时意图已经就绪
        self.shared.set_phase(Phase::Starting, None);
        Ok(())
    }

    /// 启动内核并等待进程创建完成。
    ///
    /// 失败原因取自监控线程记录的异常，与 [`reason`](Self::reason) 一致。
    pub fn start(&self, config_path: &Path, work_dir: &Path, api: &str) -> Result<()> {
        self.start_async(config_path, work_dir, api)?;

        let phase = self
            .shared
            .wait_phase(|phase| matches!(phase, Phase::Running | Phase::Stopped));
        match phase {
            Some(Phase::Running) => Ok(()),
            Some(_) => Err(anyhow!(
                "{}",
                self.reason().unwrap_or_else(|| "内核启动失败".to_string())
            )),
            // 监控线程没有响应
            None => bail!("等待内核启动超时"),
        }
    }

    /// 登记停止意图并唤醒监控线程，立即返回。
    ///
    /// 返回时进程可能尚未回收，此时 [`phase`](Self::phase) 为 [`Phase::Stopping`]；
    /// 需要确认已经停止时用 [`stop`](Self::stop)。
    pub fn stop_async(&self) {
        match self.phase() {
            Phase::Stopped | Phase::Stopping => return,
            Phase::Starting | Phase::Running => {}
        }

        {
            let mut control = self.shared.lock();
            control.intent = Intent::Stop;
        }
        self.shared.set_phase(Phase::Stopping, None);
    }

    /// 停止内核并等待进程回收完成。
    pub fn stop(&self) -> Result<()> {
        self.stop_async();

        match self.shared.wait_phase(|phase| phase == Phase::Stopped) {
            Some(_) => Ok(()),
            None => bail!("等待内核停止超时"),
        }
    }

    /// 让内核重载配置, 不重启进程。
    ///
    /// 内核按秒检查工作目录下的重载标记文件, 发现后自行删除并按新配置重建实例。
    /// 标记文件是跨平台的重载方式, Windows 无法向子进程发送 SIGHUP, 因此不依赖信号。
    ///
    /// 旧实例会随重载关闭, 其 gRPC 连接、连接订阅与测速队列随之失效;
    /// 因此这里先把客户端作废, 重载后的第一次取用会按新配置的监听地址重建。
    /// 内核未运行时返回错误, 避免调用方误以为刷新成功。
    pub fn reload(&self, work_dir: &Path) -> Result<()> {
        if !self.phase().is_running() {
            bail!("内核未运行");
        }

        let flag = work_dir.join(lib_core::constants::KERNEL_RELOAD_FILE);
        std::fs::write(&flag, [])
            .with_context(|| format!("创建内核重载标记失败: {}", flag.display()))?;

        self.shared.grpc.store(None);
        self.shared.url_test.set_grpc(None);
        self.shared.connections.subscribe(None);
        self.shared.connections.reset();
        Ok(())
    }

    /// 等待内核 API 就绪。
    ///
    /// 启动命令执行完毕不代表内核真正可用，因此按固定间隔调用内核 API 探测，
    /// 直到取得版本号才视为启动成功；进程提前退出或超时都返回失败原因。
    #[cfg(feature = "grpc")]
    pub async fn wait_ready(&self, timeout: Duration) -> Result<()> {
        let start = Instant::now();
        let mut last: Option<anyhow::Error> = None;

        loop {
            match self.phase() {
                // 进程已退出或启动失败，失败原因由监控线程写入
                Phase::Stopped => {
                    let reason = self
                        .reason()
                        .unwrap_or_else(|| "内核进程已退出".to_string());
                    bail!("内核启动失败: {reason}");
                }
                Phase::Running => {
                    if let Some(grpc) = self.grpc()? {
                        match grpc.version().await {
                            Ok(version) => {
                                log::info!("内核 API 就绪, 版本: {}", version.version);
                                return Ok(());
                            }
                            Err(error) => last = Some(error),
                        }
                    }
                }
                // 仍在启动或已在停止，继续等下一次探测
                Phase::Starting | Phase::Stopping => {}
            }

            if start.elapsed() >= timeout {
                let reason = match last {
                    Some(error) => format!("{error:#}"),
                    None => "内核 API 未响应".to_string(),
                };
                bail!("等待内核 API 就绪超时: {reason}");
            }

            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }

    /// 将 json 规则集转换为 srs 规则集。
    pub fn json_to_srs(&self, json_path: &Path, srs_path: &Path) -> Result<()> {
        let json = json_path
            .to_str()
            .with_context(|| format!("json 路径非 UTF-8: {}", json_path.display()))?;
        let srs = srs_path
            .to_str()
            .with_context(|| format!("srs 路径非 UTF-8: {}", srs_path.display()))?;

        let mut cmd = Command::new(&self.bin);
        cmd.arg("json2srs").arg("-c").arg(json).arg("-o").arg(srs);
        let status = cmd.status()?;
        check_status(status)
    }

    /// 校验配置文件能否被内核解析与构建。
    ///
    /// 等价于内核自身的 `check` 命令：完整走一遍配置解析与实例构建，随即关闭实例，
    /// 不接管流量。`work_dir` 为校验时使用的工作目录，配置中的相对路径以它为基准。
    pub fn check(&self, config_path: &Path, work_dir: &Path) -> Result<()> {
        let config = config_path
            .to_str()
            .with_context(|| format!("配置路径非 UTF-8: {}", config_path.display()))?;
        let work = work_dir
            .to_str()
            .with_context(|| format!("工作目录非 UTF-8: {}", work_dir.display()))?;

        let mut cmd = Command::new(&self.bin);
        cmd.arg("check").arg("-c").arg(config).arg("-w").arg(work);
        let status = cmd.status()?;
        check_status(status)
    }

    /// 取得 gRPC 客户端。
    ///
    /// 内核运行中时返回可用客户端，尚未构造则先按当前 API 地址构造并保存；
    /// 内核未运行时返回 `None`。
    pub fn grpc(&self) -> Result<Option<Arc<SingBoxGrpc>>> {
        if !self.phase().is_running() {
            // 内核已不在运行, 队列里的客户端与连接数据一并失效
            self.shared.url_test.set_grpc(None);
            self.shared.connections.subscribe(None);
            return Ok(None);
        }

        if let Some(grpc) = self.shared.grpc.load_full() {
            return Ok(Some(grpc));
        }

        let api = self.shared.api.load();
        let grpc = Arc::new(SingBoxGrpc::new(&api)?);
        self.shared.grpc.store(Some(grpc.clone()));
        self.shared.url_test.set_grpc(Some(grpc.clone()));
        // 客户端就绪后订阅线程才能建立连接订阅
        self.shared.connections.subscribe(Some(grpc.clone()));
        Ok(Some(grpc))
    }

    /// 测速任务队列。
    ///
    /// 内核运行中时先用 [`grpc`](Self::grpc) 把客户端交给队列，再推送节点；
    /// 队列自己按顺序执行，重复推送同一节点会被去重。
    pub fn url_test(&self) -> Arc<UrlTestQueue> {
        self.shared.url_test.clone()
    }

    /// 连接数据。
    ///
    /// 内核运行期间由常驻订阅持续接收推送，调用方读取快照即可；
    /// 内核未运行时快照为空。
    pub fn connections(&self) -> Arc<ConnectionStore> {
        self.shared.connections.clone()
    }

    /// 内核日志。
    ///
    /// 内核运行期间由常驻读取线程增量读取日志文件，调用方读取快照即可；
    /// 内核未运行时快照为空。
    pub fn logs(&self) -> Arc<LogStore> {
        self.shared.logs.clone()
    }
}

impl Drop for SingBox {
    fn drop(&mut self) {
        // 先结束进程再通知监控线程退出：`stop` 依赖监控线程处理停止意图
        let _ = self.stop();
        self.shared.lock().closed = true;
        self.shared.wake.notify_all();
    }
}

/// 监控线程：处理启停意图，并按间隔检查进程是否退出。
///
/// 创建与结束进程都在这条线程上完成，调用方只登记意图，因此界面线程永远不会被慢 IO 挡住。
fn watch(shared: Arc<Shared>, bin: String) {
    loop {
        // 1. 取出登记过的意图；实例已释放时直接退出
        let intent = {
            let mut control = shared.lock();
            if control.closed {
                return;
            }
            std::mem::take(&mut control.intent)
        };
        match intent {
            Intent::Idle => {}
            Intent::Start { config, work, api } => start(&shared, &bin, config, work, api),
            Intent::Stop => stop(&shared),
        }

        // 2. 进程自行退出时更新阶段
        poll_exit(&shared);

        // 3. 等下一次检查，或被新的意图唤醒
        //
        // 用条件等待而不是普通超时等待：意图可能在进入等待前就登记完成，
        // 此时唤醒信号已经发出，普通等待会白等到超时，启动因此最多被拖慢一个轮询周期。
        let guard = shared.lock();
        let _ = shared
            .wake
            .wait_timeout_while(guard, POLL_INTERVAL, |control| {
                !control.closed && matches!(control.intent, Intent::Idle)
            })
            .unwrap_or_else(|error| error.into_inner());
    }
}

/// 创建日志文件与子进程，成功后把阶段置为运行中。
fn start(shared: &Arc<Shared>, bin: &str, config: PathBuf, work: PathBuf, api: String) {
    match spawn(bin, &config, &work) {
        Ok(process) => {
            {
                let mut control = shared.lock();
                control.process = Some(process);
                control.work = Some(work.clone());
            }
            shared.api.store(Arc::new(api));

            // 上一轮连接、测速任务与日志都属于旧进程
            shared.invalidate();
            shared.connections.reset();
            // 内核日志按启动重写, 读取线程改为跟随本次启动的日志文件
            shared
                .logs
                .subscribe(Some(work.join(lib_core::constants::KERNEL_LOG_FILE)));

            shared.set_phase(Phase::Running, None);
        }
        Err(error) => {
            let reason = format!("{error:#}");
            log::error!("[内核] 启动失败: {reason}");
            shared.set_phase(Phase::Stopped, Some(reason));
        }
    }
}

/// 结束子进程并把阶段置为未运行。
fn stop(shared: &Arc<Shared>) {
    let process = {
        let mut control = shared.lock();
        control.work = None;
        control.process.take()
    };

    if let Some(mut process) = process
        && let Err(error) = process.kill()
    {
        // 进程句柄被丢弃时还会再尝试一次结束，因此这里只记录原因
        log::error!("singbox结束进程异常! {error}");
    }

    // 无论进程是否成功结束, 旧连接都不再可用
    shared.invalidate();
    shared.connections.reset();
    // 日志同样随内核一起作废, 读取线程保留并等待下一次启动
    shared.logs.subscribe(None);
    shared.set_phase(Phase::Stopped, None);
}

/// 检查进程是否已退出，退出时记录失败原因。
fn poll_exit(shared: &Arc<Shared>) {
    if !shared.phase().is_running() {
        return;
    }

    let ended = {
        let mut control = shared.lock();
        let Some(process) = control.process.as_mut() else {
            return;
        };

        match process.status() {
            Ok(Some(status)) => {
                control.process = None;
                let work = control.work.take();
                Some((status, work))
            }
            Ok(None) => None,
            Err(error) => {
                log::warn!("[内核] 查询进程状态失败: {error:#}");
                None
            }
        }
    };

    let Some((status, work)) = ended else {
        return;
    };

    // 读日志尾部拼出可读的失败原因, 这是慢 IO, 但发生在监控线程上
    let reason = match check_status(status) {
        Ok(()) => None,
        Err(error) => {
            log::error!("singbox执行异常! {error}");
            Some(match work {
                Some(work) => failure(&work, error.to_string()),
                None => error.to_string(),
            })
        }
    };

    shared.invalidate();
    shared.connections.reset();
    shared.set_phase(Phase::Stopped, reason);
}

/// 创建日志文件并启动子进程。
fn spawn(bin: &str, config_path: &Path, work_dir: &Path) -> Result<Process> {
    let config = config_path
        .to_str()
        .with_context(|| format!("配置路径非 UTF-8: {}", config_path.display()))?;
    let work = work_dir
        .to_str()
        .with_context(|| format!("工作目录非 UTF-8: {}", work_dir.display()))?;

    // 三个日志文件都在工作目录下, 每次启动都重新创建, 只保留本次启动的内容
    truncate(work_dir, lib_core::constants::KERNEL_LOG_FILE)?;
    let out = log_file(work_dir, lib_core::constants::KERNEL_STDOUT_FILE)?;
    let err = log_file(work_dir, lib_core::constants::KERNEL_STDERR_FILE)?;
    // 上次异常退出可能遗留重载标记, 启动前清掉, 避免内核一启动就重载
    remove_reload(work_dir);

    let mut cmd = Command::new(bin);
    // 参数一律按 key 传入, 便于手动启动内核做测试
    cmd.arg("start")
        .current_dir(work_dir)
        .arg("-c")
        .arg(config)
        .arg("-w")
        .arg(work)
        // 父进程 pid, 内核据此判断是否需要退出
        .arg("-p")
        .arg(std::process::id().to_string())
        // 重载标记文件名, 内核据此判断是否启用自动重载
        .arg("-r")
        .arg(lib_core::constants::KERNEL_RELOAD_FILE)
        .stdout(out)
        .stderr(err);

    Process::new(cmd)
}

/// 检查子进程退出码，非零视为失败。
fn check_status(status: ExitStatus) -> Result<()> {
    let code = status.code().unwrap_or(-1);
    if code == 0 {
        return Ok(());
    }
    bail!("内核执行失败, 退出码: {code}")
}

/// 进程异常时补充内核日志中的原始错误。
///
/// 内核只把可读的失败原因写进日志，进程退出码本身没有信息量，
/// 因此把日志尾部（最近几行）拼到原因后面，界面与 tooltip 可直接看到真实原因。
fn failure(work_dir: &Path, reason: String) -> String {
    match tail(work_dir) {
        Some(detail) => format!("{reason}\n{detail}"),
        None => reason,
    }
}

/// 删除工作目录下的重载标记，不存在时视为成功。
fn remove_reload(work_dir: &Path) {
    let flag = work_dir.join(lib_core::constants::KERNEL_RELOAD_FILE);
    if let Err(error) = std::fs::remove_file(&flag)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        log::warn!("删除内核重载标记失败: {error}");
    }
}

/// 读取工作目录下内核日志的尾部内容，取不到时返回 `None`。
///
/// 内核在实例创建阶段失败时只把原因写到标准输出，尚未开始写自身日志，
/// 因此按「内核日志 → 标准输出 → 异常输出」的顺序取第一个有内容的文件。
fn tail(work_dir: &Path) -> Option<String> {
    let names = [
        lib_core::constants::KERNEL_LOG_FILE,
        lib_core::constants::KERNEL_STDOUT_FILE,
        lib_core::constants::KERNEL_STDERR_FILE,
    ];

    names
        .iter()
        .filter_map(|name| std::fs::read_to_string(work_dir.join(name)).ok())
        .find_map(|content| {
            let lines: Vec<&str> = content
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .collect();
            if lines.is_empty() {
                return None;
            }

            let start = lines.len().saturating_sub(TAIL_LINES);
            Some(lines[start..].join("\n"))
        })
}

/// 以截断方式创建日志文件，保证只保留本次启动的日志。
fn log_file(work_dir: &Path, name: &str) -> Result<Stdio> {
    let path = work_dir.join(name);
    let file =
        File::create(&path).with_context(|| format!("创建内核日志文件失败: {}", path.display()))?;
    Ok(Stdio::from(file))
}

/// 清空工作目录下的日志文件, 文件不存在时直接创建。
fn truncate(work_dir: &Path, name: &str) -> Result<()> {
    log_file(work_dir, name)?;
    Ok(())
}

/// 创建内核实例，内核可执行文件位于安装目录。
pub fn create() -> Result<SingBox> {
    let directory = lib_core::directorys()?;
    SingBox::new(&directory.install)
}

/// 安装目录下的内核可执行文件路径。
pub fn bin_path() -> Result<PathBuf> {
    Ok(lib_core::directorys()?.install.join(BIN_NAME))
}
