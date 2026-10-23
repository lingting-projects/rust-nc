//! 内核实例。
//!
//! 实例本身只保留状态查询与共享数据访问器，具体职责拆到同目录的子模块：
//! `shared` 是与监控线程共享的运行状态，`lifecycle` 负责启停、重载与就绪等待，
//! `command` 负责一次性子进程命令，`watch` 是常驻监控线程，`process` 负责子进程创建与日志辅助。

mod command;
mod lifecycle;
mod process;
mod shared;
mod watch;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};

use crate::BIN_NAME;
use crate::connection::ConnectionStore;
use crate::grpc::SingBoxGrpc;
use crate::logs::LogStore;
use crate::url_test::UrlTestQueue;

use shared::Shared;

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
            .spawn(move || watch::run(watcher, command))
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

/// 创建内核实例，内核可执行文件位于安装目录。
pub fn create() -> Result<SingBox> {
    let directory = lib_core::directorys()?;
    SingBox::new(&directory.install)
}

/// 安装目录下的内核可执行文件路径。
pub fn bin_path() -> Result<PathBuf> {
    Ok(lib_core::directorys()?.install.join(BIN_NAME))
}
