mod _box;

use crate::_box::{load_lib, SingBoxJsonToSrs, SingBoxRefresh, SingBoxStart, SingBoxStop};
use library_core::core::{AnyResult, BizError};
use std::ffi::CString;
use std::os::raw::c_int;
use std::path::Path;

pub static version: &'static str = "v1.11.15";

/// 启动SingBox服务
pub fn start(config_path: &Path) -> AnyResult<()> {
    load_lib();
    let config_c_str = path_to_c_string(config_path)?;
    let result = unsafe { SingBoxStart(config_c_str.as_ptr() as *mut _) };
    check_result(result)
}

/// 刷新SingBox配置
pub fn refresh(config_path: &Path) -> AnyResult<()> {
    load_lib();
    let config_c_str = path_to_c_string(config_path)?;
    let result = unsafe { SingBoxRefresh(config_c_str.as_ptr() as *mut _) };
    check_result(result)
}

/// 停止SingBox服务
pub fn stop() -> AnyResult<()> {
    load_lib();
    let result = unsafe { SingBoxStop() };
    check_result(result)
}

/// 将JSON配置转换为SRS配置
pub fn json_to_srs(json_path: &Path, srs_path: &Path) -> AnyResult<()> {
    load_lib();
    let json_c_str = path_to_c_string(json_path)?;
    let srs_c_str = path_to_c_string(srs_path)?;

    let result =
        unsafe { SingBoxJsonToSrs(json_c_str.as_ptr() as *mut _, srs_c_str.as_ptr() as *mut _) };

    check_result(result)
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
fn check_result(result: c_int) -> AnyResult<()> {
    if result == 0 {
        Ok(())
    } else {
        Err(Box::new(BizError::OperationFailed(result)))
    }
}
