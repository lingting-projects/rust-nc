//! 启停、重载与就绪等待。
//!
//! 启停一律「登记意图 + 唤醒监控线程」：进程的创建与结束都发生在监控线程上，
//! 因此这些方法不会把界面线程挡在慢 IO 上。

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};

use super::Phase;
use super::SingBox;
use super::shared::Intent;

#[cfg(feature = "grpc")]
use std::time::Instant;

impl SingBox {
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
