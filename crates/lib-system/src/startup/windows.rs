//! windows 平台通过计划任务实现开机自启。

use std::env;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};
use encoding::{DecoderTrap, Encoding};
use log::LevelFilter;
use regex::Regex;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::Authorization::ConvertSidToStringSidW;
use windows::Win32::Security::{GetTokenInformation, TOKEN_QUERY, TOKEN_USER, TokenUser};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows::core::PWSTR;

use crate::process::Process;

/// 计划任务名称。
const TASK_NAME: &str = "LingtingNcStartup";
/// 单条系统指令的等待上限，单位: 毫秒
const TIMEOUT_MILLIS: i128 = 1000 * 3;
/// 计划任务模板，路径相对于本文件。
const TASK_TEMPLATE: &[u8] = include_bytes!("../../../../assets/startup_windows.xml");

/// 是否已注册开机自启。
pub fn is_startup() -> Result<bool> {
    let mut cmd = Command::new("powershell");
    cmd.arg("Get-ScheduledTask")
        .arg("| Select-Object TaskName, @{Name='IsEnabled'; Expression={$_.Settings.Enabled}}");
    let mut process = run(cmd)?;
    let stdout = process.out_string()?.unwrap_or_default();

    let enabled = stdout.lines().any(|line| {
        let line = line.trim();
        line.starts_with(TASK_NAME) && line.ends_with("True")
    });
    if enabled {
        return Ok(true);
    }

    log::trace!("{stdout}");
    Ok(false)
}

/// 注册开机自启，`cache_dir` 用于存放计划任务 xml。
pub fn enable(cache_dir: &Path) -> Result<bool> {
    let path = cache_dir.join("schtasks.xml");
    let path = path.to_str().context("计划任务路径非 UTF-8")?;

    let author = author()?;
    let sid = user_sid()?;
    let exe = env::current_exe().context("获取当前可执行文件失败")?;
    let bin = exe.to_str().context("可执行文件路径非 UTF-8")?;
    let worker = exe
        .parent()
        .context("未找到可执行文件所在目录")?
        .to_str()
        .context("可执行文件目录非 UTF-8")?;

    // 模板为 UTF-16BE 编码的 XML
    let template = encoding::all::UTF_16BE
        .decode(TASK_TEMPLATE, DecoderTrap::Replace)
        .map_err(|error| anyhow::anyhow!("解析计划任务模板失败: {error}"))?;
    let xml = template
        .replace("@author@", &author)
        .replace("@userid@", &sid)
        .replace("@exe@", bin)
        .replace("@worker@", worker);
    let xml = Regex::new(r"\r?\n")?.replace_all(&xml, "\r\n");
    std::fs::write(path, xml.as_bytes())
        .with_context(|| format!("写入计划任务模板失败: {path}"))?;

    let mut cmd = Command::new("powershell");
    cmd.arg("Register-ScheduledTask")
        .arg("-Xml")
        .arg("(Get-Content")
        .arg(format!("\"{path}\""))
        .arg("| Out-String)")
        .arg("-TaskName")
        .arg(TASK_NAME);

    run(cmd)?;
    Ok(true)
}

/// 取消开机自启。
pub fn disable() -> Result<bool> {
    let mut cmd = Command::new("powershell");
    cmd.arg("Unregister-ScheduledTask")
        .arg("-TaskName")
        .arg(TASK_NAME)
        .arg("-Confirm:$false");

    run(cmd)?;
    Ok(true)
}

/// 执行系统指令，非零退出码视为失败。
fn run(cmd: Command) -> Result<Process> {
    let mut process = Process::new_pipe(cmd)?;
    if !process.wait_millis(TIMEOUT_MILLIS)? {
        bail!("执行系统指令超时");
    }

    let status = process.status()?.context("未获取到指令退出状态")?;
    if status.success() {
        return Ok(process);
    }

    let code = status.code().unwrap_or(-1);
    if log::max_level() >= LevelFilter::Debug {
        log::debug!("out: {}", process.out_string()?.unwrap_or_default());
        log::debug!("err: {}", process.err_string()?.unwrap_or_default());
    }
    bail!("执行系统指令失败, 退出码: {code}")
}

/// 计划任务的作者，格式为 `域\用户名`。
fn author() -> Result<String> {
    let domain = env::var("USERDOMAIN").context("未设置 USERDOMAIN 环境变量")?;
    let username = env::var("USERNAME").context("未设置 USERNAME 环境变量")?;
    Ok(format!("{domain}\\{username}"))
}

/// 当前用户的 SID。
fn user_sid() -> Result<String> {
    let sid = unsafe {
        let mut token = HANDLE::default();
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token)?;

        let mut length = 0;
        let _ = GetTokenInformation(token, TokenUser, None, 0, &mut length);

        let mut buffer = vec![0u8; length as usize];
        GetTokenInformation(
            token,
            TokenUser,
            Some(buffer.as_mut_ptr() as *mut _),
            length,
            &mut length,
        )?;

        let token_user = &*(buffer.as_ptr() as *const TOKEN_USER);
        let mut pointer = PWSTR::default();
        ConvertSidToStringSidW(token_user.User.Sid, &mut pointer)?;
        let sid = pointer.to_string()?;
        CloseHandle(token)?;
        sid
    };

    log::debug!("sid: {sid}");
    Ok(sid)
}
