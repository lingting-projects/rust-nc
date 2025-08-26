use crate::SingBox;
use libloading::{Library, Symbol};
use library_core::app::get_app;
use library_core::core::{AnyResult, BizError};
use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, OnceLock};
use std::{env, process};

static lib_name: LazyLock<String> = LazyLock::new(|| {
    let target = env::var("TARGET").unwrap();
    if target.contains("windows") {
        "libsingbox.dll"
    } else if target.contains("apple") {
        "libsingbox.dylib"
    } else {
        "libsingbox.so"
    }
    .to_string()
});

static LIB: OnceLock<Library> = OnceLock::new();

fn load_lib() {
    LIB.get_or_init(|| {
        let app = get_app();
        let path = app.install_dir.join(lib_name);
        log::info!("load sing box lib from {}", path.display());
        unsafe { Library::new(path).expect("unload sing box lib") }
    });
}

type SingBoxStart =
    unsafe extern "system" fn(config_path_ptr: *mut c_char, work_dir_ptr: *mut c_char, work_dir_ptr: *mut c_int) -> c_int;

type SingBoxJsonToSrs =
    unsafe extern "system" fn(json_path_ptr: *mut c_char, srs_path_ptr: *mut c_char) -> c_int;

fn get<T>(symbol: &[u8]) -> Symbol<T> {
    unsafe {
        LIB.get()
            .expect("failed load LIB")
            .get(symbol)
            .expect("failed get symbol")
    }
}

pub struct LibSingBox {}

impl SingBox for LibSingBox {
    fn start(&self, config_path: &Path, work_dir: &Path) -> AnyResult<()> {
        let pid = process::id();
        let config_path_c = path_to_c_string(config_path)?;
        let work_dir_c = path_to_c_string(work_dir)?;
        let pid_c = CInt::new(pid);
        let config_path_ptr = config_path_c.as_ptr() as *mut _;
        let work_dir_ptr = work_dir_c.as_ptr() as *mut _;
        let i = unsafe {
            let s: Symbol<SingBoxStart> = get(b"SingBoxStart\0");
            s(config_path_ptr, work_dir_ptr)
        };
        check_result(i)
    }

    fn json_srs(&self, json_path: &Path, srs_path: &Path) -> AnyResult<()> {
        let json_c_str = path_to_c_string(json_path)?.as_ptr() as *mut _;
        let srs_c_str = path_to_c_string(srs_path)?.as_ptr() as *mut _;
        let i = unsafe {
            let s: Symbol<SingBoxJsonToSrs> = get(b"SingBoxJsonToSrs\0");
            s(json_c_str, srs_c_str)
        };
        check_result(i)
    }
}

// 将Path转换为CString
fn path_to_c_string(path: &Path) -> AnyResult<CString> {
    if path.as_os_str().is_empty() {
        return Err(Box::new(BizError::EmptyPath));
    }
    let path_str = path.to_str().ok_or_else(|| BizError::InvalidUtf8())?;
    CString::new(path_str).map_err(Into::into)
}

// 检查C函数返回结果
fn check_result(i: c_int) -> AnyResult<()> {
    // 为支持返回值识别,  小于0为异常, 其他为正常
    if i >= 0 {
        Ok(())
    } else {
        Err(Box::new(BizError::OperationFailed(i)))
    }
}

pub fn new() -> AnyResult<LibSingBox> {
    load_lib();
    Ok(LibSingBox {})
}
