//! 本地 worker 测试环境。
//!
//! 用 `wrangler dev` 在空闲端口启动 `bin-cloudflare`; 启动期间 wrangler 会按
//! `wrangler.toml` 的 build 命令编译 wasm 产物, 首次编译较慢。就绪以 worker 能
//! 响应请求为准, 与正式环境一致: 进程创建完不代表可用。

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::net::{Ipv4Addr, TcpListener};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use anyhow::{Context, Result, bail};
use lib_fetch::{FetchClient, FetchRequest};

use crate::target;

/// worker crate 目录名, 位于工作区 `crates` 下。
const CRATE_DIR: &str = "bin-cloudflare";
/// 测试产物目录名, 位于 `target/{profile}` 下。
const WORK_DIR: &str = "bin-tests/cloudflare";
/// wrangler 输出日志文件名。
const LOG_FILE: &str = "wrangler.log";
/// 启动超时, 首次需要编译 wasm 产物。
const START_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(900);
/// 启动轮询间隔。
const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(500);
/// 就绪探测路径, 未匹配任何路由时 worker 返回 403。
const READY_PATH: &str = "/";

/// 本地运行中的 worker。
pub struct Worker {
    /// 访问地址, 不含末尾斜杠
    base: String,
    child: Child,
}

impl Worker {
    /// 启动 worker 并等待就绪。
    ///
    /// `vars` 为注入 worker 的环境变量, 每项为 `(键, 值)`。
    pub async fn start(vars: &[(&str, &str)]) -> Result<Self> {
        let port = free_port()?;
        let inspector = free_port()?;
        let log = work_dir()?.join(LOG_FILE);

        log::info!(
            "[cloudflare] 启动 worker, 端口: {port}, 日志: {}",
            log.display()
        );
        let child = spawn(port, inspector, &log, vars)?;

        let mut worker = Self {
            base: format!("http://{}:{}", Ipv4Addr::LOCALHOST, port),
            child,
        };
        worker.wait_ready(&log).await?;
        log::info!("[cloudflare] worker 已就绪: {}", worker.base);

        Ok(worker)
    }

    /// worker 访问地址, 不含末尾斜杠。
    pub fn base(&self) -> &str {
        &self.base
    }

    /// 拼接请求地址, `path` 需以 `/` 开头。
    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    /// 结束 worker 并等待进程退出。
    ///
    /// `wrangler` 会在 `workerd` 之外再拉起若干子进程, 结束整棵树后还要按端口兜底:
    /// 测试被中断时 `Drop` 可能来不及执行, 残留的 `workerd` 会一直占着端口。
    pub fn stop(&mut self) -> Result<()> {
        let result = kill(&mut self.child);
        release(&self.base)?;
        result
    }

    /// 轮询等待 worker 就绪。
    async fn wait_ready(&mut self, log: &Path) -> Result<()> {
        let start = tokio::time::Instant::now();
        while start.elapsed() < START_TIMEOUT {
            if let Some(status) = self.child.try_wait()? {
                bail!(
                    "worker 启动失败, 退出码: {status}, 日志: {}",
                    tail(log).unwrap_or_default()
                );
            }

            if self.ready().await {
                return Ok(());
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }

        let _ = self.stop();
        bail!(
            "等待 worker 就绪超时, 日志: {}",
            tail(log).unwrap_or_default()
        )
    }

    /// 请求一次就绪探测路径, 能拿到响应即视为可用。
    async fn ready(&self) -> bool {
        let request = FetchRequest::new(self.url(READY_PATH));
        match FetchClient::new().get(request).await {
            Ok(response) => response.status > 0,
            Err(_) => false,
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        if let Err(error) = self.stop() {
            log::debug!("[cloudflare] 结束 worker 异常: {error}");
        }
    }
}

/// 启动 wrangler 进程, 输出写入日志文件。
fn spawn(port: u16, inspector: u16, log: &Path, vars: &[(&str, &str)]) -> Result<Child> {
    let mut cmd = Command::new(npx());
    cmd.current_dir(crate_dir())
        .arg("wrangler")
        .arg("dev")
        .arg("--ip")
        .arg(Ipv4Addr::LOCALHOST.to_string())
        .arg("--port")
        .arg(port.to_string())
        .arg("--inspector-port")
        .arg(inspector.to_string())
        .arg("--log-level")
        .arg("warn")
        .arg("--show-interactive-dev-session=false")
        .stdout(log_stdio(log)?)
        .stderr(log_stdio(log)?)
        .stdin(Stdio::null());

    for (key, value) in vars {
        cmd.arg("--var").arg(format!("{key}:{value}"));
    }

    cmd.spawn()
        .with_context(|| format!("启动 wrangler 失败, 日志: {}", log.display()))
}

/// npx 可执行文件名, windows 下需要带扩展名。
fn npx() -> &'static str {
    match cfg!(windows) {
        true => "npx.cmd",
        false => "npx",
    }
}

/// worker crate 目录。
fn crate_dir() -> PathBuf {
    target::workspace_root().join("crates").join(CRATE_DIR)
}

/// 测试产物目录, 每次重建, 避免上一次的日志干扰排查。
fn work_dir() -> Result<PathBuf> {
    let dir = target::profile_dir()?.join(WORK_DIR);
    if dir.exists() {
        std::fs::remove_dir_all(&dir)
            .with_context(|| format!("清理 worker 工作目录失败: {}", dir.display()))?;
    }
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("创建 worker 工作目录失败: {}", dir.display()))?;

    Ok(dir)
}

/// 以追加方式打开日志文件并转为子进程输出。
fn log_stdio(path: &Path) -> Result<Stdio> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("创建 wrangler 日志失败: {}", path.display()))?;

    Ok(Stdio::from(file))
}

/// 读取日志末尾若干字节, 便于失败时定位。
fn tail(path: &Path) -> Option<String> {
    const LIMIT: u64 = 4096;

    let mut file = File::open(path).ok()?;
    let length = file.metadata().ok()?.len();
    file.seek(SeekFrom::Start(length.saturating_sub(LIMIT)))
        .ok()?;

    let mut buffer = String::new();
    file.read_to_string(&mut buffer).ok()?;
    Some(buffer)
}

/// 申请一个空闲端口。
fn free_port() -> Result<u16> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).context("申请空闲端口失败")?;
    Ok(listener.local_addr().context("读取监听地址失败")?.port())
}

/// 结束进程, windows 下连同子进程一起结束。
///
/// `wrangler` 会另外拉起 `workerd`, 只结束父进程会留下监听端口的孤儿进程。
fn kill(child: &mut Child) -> Result<()> {
    if child.try_wait()?.is_some() {
        return Ok(());
    }

    #[cfg(windows)]
    {
        let status = Command::new("taskkill")
            .args(["/T", "/F", "/PID", &child.id().to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .context("结束 worker 进程失败")?;
        if !status.success() {
            child.kill()?;
        }
    }

    #[cfg(not(windows))]
    child.kill()?;

    let _ = child.wait();
    Ok(())
}

/// 按端口结束仍在监听 worker 的进程, 用于清理被中断的测试留下的孤儿 `workerd`。
fn release(base: &str) -> Result<()> {
    let Some(port) = base
        .rsplit(':')
        .next()
        .and_then(|port| port.parse::<u16>().ok())
    else {
        return Ok(());
    };

    for pid in listeners(port)? {
        log::warn!("[cloudflare] 端口 {port} 仍被占用, 结束进程: {pid}");
        let _ = Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }

    Ok(())
}

/// 监听指定端口的进程 id。
fn listeners(port: u16) -> Result<Vec<u32>> {
    let pattern = format!(":{port} ");
    let mut pids: Vec<u32> = Vec::new();
    for line in netstat()?.lines() {
        // 只关心监听状态, 其他状态是已建立或待关闭的连接
        if !line.contains(&pattern) || !line.contains("LISTENING") {
            continue;
        }

        let Some(pid) = line
            .split_whitespace()
            .last()
            .and_then(|pid| pid.parse().ok())
        else {
            continue;
        };
        if !pids.contains(&pid) {
            pids.push(pid);
        }
    }

    Ok(pids)
}

/// 本机 tcp 监听表。
fn netstat() -> Result<String> {
    let output = Command::new("netstat")
        .args(["-ano", "-p", "TCP"])
        .output()
        .context("读取端口占用失败")?;

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
