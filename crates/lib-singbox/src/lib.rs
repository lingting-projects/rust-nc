use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex, MutexGuard};

use anyhow::{Context, Result, bail};
use arc_swap::ArcSwapOption;
use lib_system::process::Process;

#[cfg(feature = "grpc")]
use std::time::{Duration, Instant};

mod connection;
mod grpc;
mod logs;
mod url_test;

pub use connection::{ConnectionInfo, ConnectionRecord, ConnectionSnapshot, ConnectionStore};
pub use grpc::{Group, GroupItem, ProcessInfo, SingBoxGrpc};
pub use logs::{Level as LogLevel, LogBlock, LogSnapshot, LogStore};
pub use url_test::UrlTestQueue;

/// 随包分发的 sing-box 版本。
pub const VERSION: &str = "v1.14.3";

/// 内核进程可执行文件名，位于安装目录下。
pub const BIN_NAME: &str = "lingting-nc-singbox";

/// 内核异常时附加到失败原因中的日志行数。
const TAIL_LINES: usize = 10;

/// 内核运行状态。
#[derive(Debug, Clone, Default)]
pub struct State {
    /// 是否正在运行
    pub running: bool,
    /// 是否发生异常
    pub error: bool,
    /// 异常原因
    pub reason: Option<String>,
}

/// 内核实例。
///
/// 仅支持随包分发的二进制内核，实例创建时即绑定可执行文件路径，
/// 之后通过 [`SingBox::start`] 与 [`SingBox::stop`] 管理其生命周期。
/// 内部状态受锁保护，实例可由多个线程共享。
pub struct SingBox {
    bin: String,
    inner: Mutex<Inner>,
    /// gRPC 客户端，仅在内核运行期间存在。
    grpc: ArcSwapOption<SingBoxGrpc>,
    /// 测速任务队列，随内核对象一起创建，内核启停时注入或清空客户端。
    url_test: Arc<UrlTestQueue>,
    /// 连接数据，内核运行期间由常驻订阅把推送累积成快照。
    connections: Arc<ConnectionStore>,
    /// 内核日志，内核运行期间由常驻读取线程把日志文件解析成块。
    logs: Arc<LogStore>,
}

/// 内核实例的内部状态。
struct Inner {
    process: Option<Process>,
    error: bool,
    reason: Option<String>,
    /// 内核 API 监听地址，用于构造 gRPC 客户端。
    api: String,
    /// 内核工作目录，进程异常时据此读取日志尾部的原始错误。
    work: PathBuf,
}

impl SingBox {
    /// 创建实例，`bin_dir` 为内核可执行文件所在目录。
    pub fn new(bin_dir: impl AsRef<Path>) -> Result<Self> {
        let bin = bin_dir.as_ref().join(BIN_NAME);
        let bin = bin
            .to_str()
            .with_context(|| format!("内核路径非 UTF-8: {}", bin.display()))?
            .to_string();
        log::info!("load sing box bin from {bin}");
        Ok(Self {
            bin,
            inner: Mutex::new(Inner {
                process: None,
                error: false,
                reason: None,
                api: String::new(),
                work: PathBuf::new(),
            }),
            grpc: ArcSwapOption::empty(),
            url_test: Arc::new(UrlTestQueue::new()),
            connections: Arc::new(ConnectionStore::new()),
            logs: Arc::new(LogStore::new()),
        })
    }

    /// 查询当前状态。
    pub fn state(&self) -> Result<State> {
        let mut inner = self.lock();
        if let Some(process) = &mut inner.process {
            // 已经结束时判断结果, 未结束不做处理
            if let Some(status) = process.status()? {
                inner.process = None;
                // 内核退出后 gRPC 连接已失效, 一并清理
                self.grpc.store(None);
                self.url_test.set_grpc(None);
                self.connections.subscribe(None);
                if let Err(error) = Self::status(status) {
                    log::error!("singbox执行异常! {error}");
                    inner.error = true;
                    inner.reason = Some(failure(&inner.work, error.to_string()));
                }
            }
        }

        Ok(State {
            running: inner.process.is_some(),
            error: inner.error,
            reason: inner.reason.clone(),
        })
    }

    /// 以 `config_path` 为配置、`work_dir` 为工作目录启动内核，`api` 为内核 API 监听地址。
    pub fn start(&self, config_path: &Path, work_dir: &Path, api: &str) -> Result<()> {
        let mut inner = self.lock();
        if inner.process.is_some() {
            return Ok(());
        }

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
        let mut cmd = Command::new(&self.bin);
        cmd.arg("start")
            .current_dir(work_dir)
            .arg(config)
            .arg(work)
            // 父进程 pid, 内核据此判断是否需要退出
            .arg(std::process::id().to_string())
            .stdout(out)
            .stderr(err);
        inner.process = Some(Process::new(cmd)?);
        inner.api = api.to_string();
        inner.work = work_dir.to_path_buf();
        inner.error = false;
        inner.reason = None;
        // 上一轮连接属于旧进程, 启动新进程时丢弃
        self.grpc.store(None);
        self.url_test.set_grpc(None);
        self.url_test.clear();
        // 上一轮连接数据同样属于旧进程
        self.connections.subscribe(None);
        self.connections.reset();
        // 内核日志同样按启动重写, 读取线程改为跟随本次启动的日志文件
        self.logs
            .subscribe(Some(work_dir.join(lib_core::constants::KERNEL_LOG_FILE)));
        Ok(())
    }

    /// 停止内核。
    pub fn stop(&self) -> Result<()> {
        // 无论进程是否成功结束, 旧连接都不再可用
        self.grpc.store(None);
        // 内核已停止, 排队中的测速不可能成功, 直接丢弃
        self.url_test.set_grpc(None);
        self.url_test.clear();
        // 连接数据随内核一起作废, 订阅线程保留并等待下一次启动
        self.connections.subscribe(None);
        self.connections.reset();
        // 日志同样随内核一起作废, 读取线程保留并等待下一次启动
        self.logs.subscribe(None);
        let Some(mut process) = self.lock().process.take() else {
            return Ok(());
        };

        match process.kill() {
            Ok(()) => Ok(()),
            Err(error) => {
                // 没结束, 放回去
                self.lock().process = Some(process);
                log::error!("singbox结束进程异常! {error}");
                Err(error)
            }
        }
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
            let state = self.state()?;
            if !state.running {
                let reason = state.reason.unwrap_or_else(|| "内核进程已退出".to_string());
                bail!("内核启动失败: {reason}");
            }

            if let Some(grpc) = self.grpc()? {
                match grpc.version().await {
                    Ok(version) => {
                        log::info!("内核 API 就绪, 版本: {}", version.version);
                        return Ok(());
                    }
                    Err(error) => last = Some(error),
                }
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
        cmd.arg("json2srs").arg(json).arg(srs);
        let status = cmd.status()?;
        Self::status(status)
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
        cmd.arg("check").arg(config).arg(work);
        let status = cmd.status()?;
        Self::status(status)
    }

    /// 取得 gRPC 客户端。
    ///
    /// 内核运行中时返回可用客户端，尚未构造则先按当前 API 地址构造并保存；
    /// 内核未运行时返回 `None`。
    pub fn grpc(&self) -> Result<Option<Arc<SingBoxGrpc>>> {
        let api = {
            let inner = self.lock();
            if inner.process.is_none() {
                // 内核已不在运行, 队列里的客户端也一并失效
                // 内核已不在运行, 队列里的客户端与连接数据一并失效
                self.url_test.set_grpc(None);
                self.connections.subscribe(None);
                return Ok(None);
            }
            inner.api.clone()
        };

        if let Some(grpc) = self.grpc.load_full() {
            return Ok(Some(grpc));
        }

        let grpc = Arc::new(SingBoxGrpc::new(&api)?);
        self.grpc.store(Some(grpc.clone()));
        self.url_test.set_grpc(Some(grpc.clone()));
        // 客户端就绪后订阅线程才能建立连接订阅
        self.connections.subscribe(Some(grpc.clone()));
        Ok(Some(grpc))
    }

    /// 测速任务队列。
    ///
    /// 内核运行中时先用 [`grpc`](Self::grpc) 把客户端交给队列，再推送节点；
    /// 队列自己按顺序执行，重复推送同一节点会被去重。
    pub fn url_test(&self) -> Arc<UrlTestQueue> {
        self.url_test.clone()
    }

    /// 连接数据。
    ///
    /// 内核运行期间由常驻订阅持续接收推送，调用方读取快照即可；
    /// 内核未运行时快照为空。
    pub fn connections(&self) -> Arc<ConnectionStore> {
        self.connections.clone()
    }

    /// 内核日志。
    ///
    /// 内核运行期间由常驻读取线程增量读取日志文件，调用方读取快照即可；
    /// 内核未运行时快照为空。
    pub fn logs(&self) -> Arc<LogStore> {
        self.logs.clone()
    }
    /// 取内部状态，锁被污染时继续使用其内容。
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|error| error.into_inner())
    }

    /// 检查子进程退出码，非零视为失败。
    fn status(status: ExitStatus) -> Result<()> {
        let code = status.code().unwrap_or(-1);
        if code == 0 {
            return Ok(());
        }
        bail!("内核执行失败, 退出码: {code}")
    }
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
impl Drop for SingBox {
    fn drop(&mut self) {
        let _ = self.stop();
    }
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
