mod _box;

use crate::_box::{load_lib, SingBoxJsonToSrs, SingBoxRunning, SingBoxStart, SingBoxStop};
use library_core::core::{AnyResult, BizError};
use std::ffi::CString;
use std::os::raw::c_int;
use std::path::Path;

pub static version: &'static str = "v1.11.9";

pub fn is_running() -> AnyResult<bool> {
    load_lib();
    let i = SingBoxRunning();
    check_result(i)?;
    Ok(i == 1)
}

/// 启动SingBox服务
pub fn start(config_path: &Path, work_dir: &Path) -> AnyResult<()> {
    load_lib();
    let config_path_c = path_to_c_string(config_path)?;
    let work_dir_c = path_to_c_string(work_dir)?;
    let config_path_ptr = config_path_c.as_ptr() as *mut _;
    let work_dir_ptr = work_dir_c.as_ptr() as *mut _;
    let i = SingBoxStart(config_path_ptr, work_dir_ptr);
    check_result(i)
}

/// 停止SingBox服务
pub fn stop() -> AnyResult<()> {
    load_lib();
    let i = SingBoxStop();
    check_result(i)
}

/// 将JSON配置转换为SRS配置
pub fn json_to_srs(json_path: &Path, srs_path: &Path) -> AnyResult<()> {
    load_lib();
    let json_c_str = path_to_c_string(json_path)?;
    let srs_c_str = path_to_c_string(srs_path)?;
    let i = SingBoxJsonToSrs(json_c_str.as_ptr() as *mut _, srs_c_str.as_ptr() as *mut _);
    check_result(i)
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
