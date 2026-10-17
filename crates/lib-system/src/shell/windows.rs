use std::path::Path;

use anyhow::{Context, Result, bail};
use windows::Win32::Foundation::{HWND, RPC_E_CHANGED_MODE};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoCreateInstance, CoInitializeEx,
    CoUninitialize,
};
use windows::Win32::System::Ole::IEnumVARIANT;
use windows::Win32::System::Variant::{VARIANT, VT_DISPATCH, VariantClear};
use windows::Win32::UI::Shell::{
    Common::ITEMIDLIST, ILCreateFromPathW, ILFree, ILIsEqual, IPersistFolder2, IShellBrowser,
    IShellView, IShellWindows, ShellExecuteW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    IsIconic, SW_RESTORE, SW_SHOWNORMAL, SetForegroundWindow, ShowWindow,
};
use windows::core::{GUID, HSTRING, Interface, PCWSTR, w};

/// `IShellWindows` 的类标识符，`windows` crate 未导出该常量。
const CLSID_SHELL_WINDOWS: GUID = GUID::from_u128(0x9ba05972_f6a8_11cf_a442_00a0c90a8f39);

/// 打开目标路径。
///
/// 先在所有资源管理器窗口中查找已经位于该目录的窗口：找到就直接激活，
/// 这样同一目录不会因为反复点击而开出多个窗口；
/// 没找到时交给系统打开，由 Windows 11 的多标签资源管理器复用已有窗口。
pub fn open_path(path: &Path) -> Result<()> {
    if !path.exists() {
        bail!("路径不存在: {}", path.display());
    }

    // 窗口枚举依赖 COM，COM 不可用时退化为交给系统打开。
    match Com::new() {
        Ok(_com) => {
            let found = find_window(path).unwrap_or_else(|error| {
                log::debug!("查找已打开的资源管理器窗口失败: {error:#}");
                None
            });
            if let Some(hwnd) = found {
                activate(hwnd);
                return Ok(());
            }
        }
        Err(error) => log::debug!("初始化 COM 失败, 直接交给系统打开: {error:#}"),
    }

    open_by_system(path)
}

/// 让系统按默认方式打开路径。
fn open_by_system(path: &Path) -> Result<()> {
    let target = HSTRING::from(path.as_os_str());
    let hinstance = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            &target,
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    // `ShellExecuteW` 的返回值大于 32 表示成功，其余为错误码。
    let code = hinstance.0 as usize;
    if code <= 32 {
        bail!(
            "打开目录失败, ShellExecuteW 返回 {code}: {}",
            path.display()
        );
    }
    Ok(())
}

/// 查找正在显示目标目录的资源管理器窗口。
fn find_window(path: &Path) -> Result<Option<HWND>> {
    let pidl = Pidl::new(path)?;
    let shell_windows: IShellWindows =
        unsafe { CoCreateInstance(&CLSID_SHELL_WINDOWS, None, CLSCTX_ALL)? };
    let windows = unsafe { shell_windows._NewEnum()?.cast::<IEnumVARIANT>()? };

    loop {
        let mut item = [VARIANT::default()];
        let mut fetched = 0u32;
        let _ = unsafe { windows.Next(&mut item, &mut fetched) };
        if fetched == 0 {
            return Ok(None);
        }

        let hwnd = match_window(&item[0], &pidl);
        // 枚举项持有一份接口引用，用完由 `VariantClear` 释放。
        let _ = unsafe { VariantClear(&mut item[0]) };
        if hwnd.is_some() {
            return Ok(hwnd);
        }
    }
}

/// 单个资源管理器窗口命中目标目录时返回它的窗口句柄。
fn match_window(item: &VARIANT, pidl: &Pidl) -> Option<HWND> {
    // 枚举项是 `VT_DISPATCH`，指向浏览器窗口。
    if unsafe { item.Anonymous.Anonymous.vt } != VT_DISPATCH {
        return None;
    }
    let dispatch = unsafe { item.Anonymous.Anonymous.Anonymous.pdispVal.clone() };
    let dispatch = core::mem::ManuallyDrop::into_inner(dispatch)?;
    let browser = dispatch.cast::<IShellBrowser>().ok()?;
    let hwnd = unsafe { browser.GetWindow() }.ok()?;
    if hwnd.is_invalid() {
        return None;
    }

    let view = unsafe { browser.QueryActiveShellView() }.ok()?;
    let folder = view.cast::<IPersistFolder2>().ok()?;
    let current = unsafe { folder.GetCurFolder() }.ok()?;
    if current.is_null() {
        return None;
    }

    // `GetCurFolder` 分配的 pidl 由调用方释放。
    let equal = unsafe { ILIsEqual(current, pidl.0) }.as_bool();
    unsafe { ILFree(Some(current)) };
    equal.then_some(hwnd)
}

/// 把窗口切到前台；最小化的窗口先还原，否则只能闪一下任务栏。
fn activate(hwnd: HWND) {
    if unsafe { IsIconic(hwnd) }.as_bool() {
        let _ = unsafe { ShowWindow(hwnd, SW_RESTORE) };
    }
    // 前台切换由系统裁决，返回 `false` 表示被系统拒绝，此处无法补救。
    let _ = unsafe { SetForegroundWindow(hwnd) };
}

/// 目标路径的 pidl，析构时释放。
struct Pidl(*mut ITEMIDLIST);

impl Pidl {
    fn new(path: &Path) -> Result<Self> {
        let target = HSTRING::from(path.as_os_str());
        let pidl = unsafe { ILCreateFromPathW(&target) };
        if pidl.is_null() {
            bail!("解析路径失败: {}", path.display());
        }
        Ok(Self(pidl))
    }
}

impl Drop for Pidl {
    fn drop(&mut self) {
        unsafe { ILFree(Some(self.0)) };
    }
}

/// COM 运行环境：`CoInitializeEx` 成功后由本结构负责反初始化。
struct Com {
    /// 本次调用是否真正完成了初始化；`RPC_E_CHANGED_MODE` 时线程已被别的套间占用。
    initialized: bool,
}

impl Com {
    fn new() -> Result<Self> {
        // 与界面线程共用一个 STA 套间，避免跨套间访问 Shell 对象。
        let result =
            unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
        if result == RPC_E_CHANGED_MODE {
            log::debug!("当前线程已使用其它 COM 套间, 复用现有套间");
            return Ok(Self { initialized: false });
        }
        result.ok().context("初始化 COM 失败")?;
        Ok(Self { initialized: true })
    }
}

impl Drop for Com {
    fn drop(&mut self) {
        if self.initialized {
            unsafe { CoUninitialize() };
        }
    }
}
