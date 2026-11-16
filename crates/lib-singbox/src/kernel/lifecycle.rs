//! 启停、重载与就绪等待。
//!
//! 启停一律「登记意图 + 唤醒监控线程」：进程的创建与结束都发生在监控线程上，
//! 因此这些方法不会把界面线程挡在慢 IO 上。

use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};

use super::Phase;
use super::SingBox;
use super::shared::Intent;

#[cfg(feature = "grpc")]
use std::time::Instant;

/// 等待内核 API 就绪超时。
///
/// 与「进程提前退出」区分开：超时说明进程仍然存活、只是没在时限内就绪，
/// 调用方可以据此重试，而不是直接判定内核不可用。
#[cfg(feature = "grpc")]
#[derive(Debug)]
pub struct WaitTimeout {
    /// 超时前最后一次探测的失败原因。
    pub reason: String,
}

#[cfg(feature = "grpc")]
impl std::fmt::Display for WaitTimeout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "等待内核 API 就绪超时: {}", self.reason)
    }
}

#[cfg(feature = "grpc")]
impl std::error::Error for WaitTimeout {}

impl SingBox {
    /// 登记启动意图并唤醒监控线程，立即返回。
    ///
    /// 返回时阶段为 [`Phase::Spawn`]，进程由监控线程创建，创建完成后转为
    /// [`Phase::Starting`]；内核真正可用的标志是 API 探测通过，见
    /// [`wait_ready`](Self::wait_ready)。
    pub fn start_async(&self, config_path: &Path, work_dir: &Path, api: &str) -> Result<()> {
        match self.phase() {
            // 已在启动或已运行，重复登记没有意义
            Phase::Spawn | Phase::Starting | Phase::Running | Phase::Reloading => return Ok(()),
            Phase::Stopping => bail!("内核正在停止"),
            Phase::Stopped => {}
        }

        // API 地址在登记意图时就写入：启动期间监控线程创建进程后即可据此构造客户端探测就绪
        self.shared.api.store(Arc::new(api.to_string()));
        {
            let mut control = self.shared.lock();
            control.intent = Intent::Start {
                config: config_path.to_path_buf(),
                work: work_dir.to_path_buf(),
            };
        }
        // 先登记意图再置阶段：监控线程被唤醒时意图已经就绪
        self.shared.set_phase(Phase::Spawn, None);
        Ok(())
    }

    /// 启动内核并等待进程创建完成。
    ///
    /// 成功只表示进程已经创建、阶段进入 [`Phase::Starting`]，内核 API 是否可用
    /// 还要由 [`wait_ready`](Self::wait_ready) 验证。
    /// 失败原因取自监控线程记录的异常，与 [`reason`](Self::reason) 一致。
    pub fn start(&self, config_path: &Path, work_dir: &Path, api: &str) -> Result<()> {
        self.start_async(config_path, work_dir, api)?;

        let phase = self
            .shared
            .wait_phase(|phase| matches!(phase, Phase::Starting | Phase::Running | Phase::Stopped));
        match phase {
            Some(Phase::Starting | Phase::Running) => Ok(()),
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
            // 重载期间进程还在, 同样允许停止
            Phase::Spawn | Phase::Starting | Phase::Running | Phase::Reloading => {}
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
    /// 只登记重载意图, 由监控线程写重载标记并等待结果: 内核按秒检查工作目录下的标记
    /// 文件, 发现后自行删除并按新配置重建实例。标记文件是跨平台的重载方式,
    /// Windows 无法向子进程发送 SIGHUP, 因此不依赖信号。
    ///
    /// 返回时阶段为 [`Phase::Reloading`]: 重载是异步的, 标记写完到新实例接管流量之间
    /// 有一到两秒的窗口, 旧实例的 api 连接是 h2c 劫持连接, 实例关闭后连接仍然存活但
    /// grpc 服务已停止, 此时建连会得到只回 200 且没有 grpc-status 的僵尸连接且无法
    /// 自愈。因此重载期间不提供客户端, 内核给出重载完成标记后才回到 [`Phase::Running`]。
    /// 内核未运行时返回错误, 避免调用方误以为刷新成功。
    pub fn reload(&self) -> Result<()> {
        match self.phase() {
            Phase::Running => {
                {
                    let mut control = self.shared.lock();
                    control.intent = Intent::Reload;
                }
                // 先登记意图再置阶段: 监控线程被唤醒时意图已经就绪, 取客户端的地方立刻拿不到客户端
                self.shared.set_phase(Phase::Reloading, None);
            }
            // 重载中再来的请求在当前这一轮结束后立刻补一轮, 免得新配置被丢掉
            Phase::Reloading => self.shared.lock().reload_pending = true,
            Phase::Spawn | Phase::Starting => bail!("内核正在启动"),
            Phase::Stopped | Phase::Stopping => bail!("内核未运行"),
        }

        Ok(())
    }

    /// 等待内核 API 就绪。
    ///
    /// 启动命令执行完毕不代表内核真正可用，因此按固定间隔调用内核 API 探测，
    /// 直到取得版本号才把阶段置为运行中；进程提前退出或超时都返回失败原因。
    ///
    /// 整个探测期间阶段保持 [`Phase::Starting`]：只有 API 验证通过才算启动完成，
    /// 进程存在但 API 未响应的内核仍然不可用。超时视为启动失败，由调用方停止进程；
    /// 超时以 [`WaitTimeout`] 返回，与进程提前退出区分开，调用方可据此重试。
    #[cfg(feature = "grpc")]
    pub async fn wait_ready(&self, timeout: Duration) -> Result<()> {
        let start = Instant::now();
        let mut last: Option<anyhow::Error> = None;
        // 探测失败的内核只在界面上体现为超时, 因此把每次探测的失败原因与阶段记进日志
        let mut probe = 0u64;
        log::info!(
            "[内核] 等待 API 就绪, 地址: {}, 超时: {:?}",
            self.shared.api.load(),
            timeout
        );

        loop {
            match self.phase() {
                // 进程已退出或启动失败，失败原因由监控线程写入
                Phase::Stopped => {
                    let reason = self
                        .reason()
                        .unwrap_or_else(|| "内核进程已退出".to_string());
                    log::error!("[内核] 启动期间进程已停止: {reason}");
                    bail!("内核启动失败: {reason}");
                }
                // 进程已创建（或已验证就绪），探测一次 API
                Phase::Starting | Phase::Running => {
                    if let Some(grpc) = self.grpc()? {
                        probe += 1;
                        match grpc.version().await {
                            Ok(version) => {
                                log::info!("内核 API 就绪, 版本: {}", version.version);
                                self.shared.set_phase(Phase::Running, None);
                                return Ok(());
                            }
                            Err(error) => {
                                // 首次探测必然失败, 每 5 次(约 1 秒)记录一次即可
                                if probe == 1 || probe.is_multiple_of(5) {
                                    log::debug!("[内核] API 探测失败, 第 {probe} 次: {error:#}");
                                }
                                last = Some(error);
                            }
                        }
                    }
                }
                // 进程尚未创建、正在停止或正在重载实例，继续等下一次探测
                Phase::Spawn | Phase::Stopping | Phase::Reloading => {}
            }

            if start.elapsed() >= timeout {
                let reason = match last {
                    Some(error) => format!("{error:#}"),
                    None => "内核 API 未响应".to_string(),
                };
                log::error!(
                    "[内核] 等待 API 就绪超时, 探测次数: {probe}, 阶段: {:?}, 原因: {reason}",
                    self.phase()
                );
                return Err(WaitTimeout { reason }.into());
            }

            tokio::time::sleep(Duration::from_millis(200)).await;
        }
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
        super::command::check_status(status)
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
        super::command::check_status(status)
    }
}
