//! 子进程创建与日志辅助。

use std::fs::File;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result};
use lib_system::process::Process;

/// 内核异常时附加到失败原因中的日志行数。
const TAIL_LINES: usize = 10;

/// 创建日志文件并启动子进程。
pub(super) fn spawn(bin: &str, config_path: &Path, work_dir: &Path) -> Result<Process> {
    let config = config_path
        .to_str()
        .with_context(|| format!("配置路径非 UTF-8: {}", config_path.display()))?;
    let work = work_dir
        .to_str()
        .with_context(|| format!("工作目录非 UTF-8: {}", work_dir.display()))?;

    log::info!(
        "[内核] 启动进程: bin={bin}, config={config}, work={work}, 管理员权限={:?}",
        lib_system::is_elevated()
    );
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

    match Process::new(cmd) {
        Ok(process) => {
            log::info!("[内核] 进程已创建");
            Ok(process)
        }
        Err(error) => {
            log::error!("[内核] 创建进程失败: {error:#}");
            Err(error)
        }
    }
}

/// 进程异常时补充内核日志中的原始错误。
///
/// 内核只把可读的失败原因写进日志，进程退出码本身没有信息量，
/// 因此把日志尾部（最近几行）拼到原因后面，界面与 tooltip 可直接看到真实原因。
pub(super) fn failure(work_dir: &Path, reason: String) -> String {
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
