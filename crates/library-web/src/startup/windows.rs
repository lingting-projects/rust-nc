use encoding::{DecoderTrap, Encoding};
use library_core::app::get_app;
use library_core::core::{current_millis, AnyResult, BizError};
use library_core::file;
use library_core::logger::is_enable_debug;
use library_core::system::process::Process;
use library_sing_box::State;
use regex::Regex;
use std::env;
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, SystemTime};
use windows::{
    core::PWSTR,
    Win32::{
        Foundation::{CloseHandle, HANDLE},
        Security::{
            Authorization::ConvertSidToStringSidW, GetTokenInformation, TokenUser, TOKEN_QUERY,
            TOKEN_USER,
        },
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    },
};

static name: &'static str = "LingtingNcStartup";
static timeout: i128 = 1000 * 3;

fn run_timeout(cmd: Command) -> AnyResult<Process> {
    let mut process = Process::new_pipe(cmd)?;
    let wait = process.wait_millis(timeout)?;
    if !wait {
        return Err(Box::new(BizError::Timeout));
    }
    let status = process.status()?.unwrap();
    if !status.success() {
        let code = status.code().unwrap_or(-1);
        log::error!("执行指令异常! {}", code);
        if is_enable_debug() {
            let out = process.out_string()?.unwrap_or_else(|| "".into());
            log::debug!("out: {}", out);
            let err = process.err_string()?.unwrap_or_else(|| "".into());
            log::debug!("err: {}", err);
        }
        return Err(Box::new(BizError::OperationFailed(code)));
    }
    Ok(process)
}

pub fn is_startup() -> AnyResult<bool> {
    let mut cmd = Command::new("powershell");
    cmd.arg("Get-ScheduledTask")
        .arg("| Select-Object TaskName, @{Name='IsEnabled'; Expression={$_.Settings.Enabled}}");
    let mut process = run_timeout(cmd)?;
    let stdout = process.out_string()?.unwrap_or_else(|| "".into());
    let enabled = stdout.lines().any(|l| {
        let t = l.trim();
        t.starts_with(name) && t.ends_with("True")
    });
    if enabled {
        return Ok(true);
    }

    log::trace!("{}", stdout);
    Ok(false)
}

fn author() -> AnyResult<String> {
    let domain = env::var("USERDOMAIN")?;
    let username = env::var("USERNAME")?;
    Ok(format!("{}\\{}", domain, username))
}

fn user_sid() -> AnyResult<String> {
    // 获取当前进程句柄
    let sid = unsafe {
        // 打开当前进程的访问令牌
        let process = GetCurrentProcess();
        let mut token_handle = HANDLE::default();
        OpenProcessToken(process, TOKEN_QUERY, &mut token_handle)?;

        // 第一次调用获取所需缓冲区大小
        let mut return_length = 0;
        let _ = GetTokenInformation(token_handle, TokenUser, None, 0, &mut return_length);

        // 分配缓冲区
        let mut buffer = vec![0u8; return_length as usize];

        // 再次调用获取 TOKEN_USER
        GetTokenInformation(
            token_handle,
            TokenUser,
            Some(buffer.as_mut_ptr() as *mut _),
            return_length,
            &mut return_length,
        )?;

        // 取出 TOKEN_USER 结构体
        let token_user = &*(buffer.as_ptr() as *const TOKEN_USER);
        let sid_ptr = token_user.User.Sid;

        // 将 SID 转换为字符串形式
        let mut string_sid_ptr = PWSTR::default();
        ConvertSidToStringSidW(sid_ptr, &mut string_sid_ptr)?;

        // 转为 Rust 字符串
        let sid_str = string_sid_ptr.to_string()?;
        log::debug!("sid: {}", &sid_str);
        // 关闭句柄
        CloseHandle(token_handle)?;
        sid_str
    };

    Ok(sid)
}

pub fn enable() -> AnyResult<bool> {
    let app = get_app();
    let _path = app.cache_dir.join("schtasks.xml");
    let path = _path.to_str().expect("failed get schtasks path");

    let author = author()?;
    let user_sid = user_sid()?;
    let _exe = env::current_exe()?;
    let bin = _exe.to_str().expect("get exe path err");
    let worker = _exe
        .parent()
        .expect("get exe dir err")
        .to_str()
        .expect("get exe dir path err");
    let bytes = include_bytes!("../../../../assets/startup_windows.xml");
    let template = encoding::all::UTF_16BE.decode(bytes, DecoderTrap::Replace)?;

    let xml = template
        .replace("@author@", &author)
        .replace("@userid@", &user_sid)
        .replace("@exe@", &bin)
        .replace("@worker@", &worker);
    let re = Regex::new(r"\r?\n")?;
    let xml_crlf = re.replace_all(&xml, "\r\n");

    file::overwrite(path, &xml_crlf)?;

    let mut cmd = Command::new("powershell");

    cmd.arg("Register-ScheduledTask")
        .arg("-Xml")
        .arg("(Get-Content")
        .arg(format!("\"{}\"", path))
        .arg("| Out-String)")
        .arg("-TaskName")
        .arg(name);

    run_timeout(cmd)?;
    Ok(true)
}

pub fn disable() -> AnyResult<bool> {
    let mut cmd = Command::new("powershell");
    cmd.arg("Unregister-ScheduledTask")
        .arg("-TaskName")
        .arg(name)
        .arg("-Confirm:$false");

    run_timeout(cmd)?;
    Ok(true)
}
