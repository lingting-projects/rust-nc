use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

static lib: OnceLock<Library> = OnceLock::new();

pub fn load_lib() {
    lib.get_or_init(|| {
        // 获取嵌入的DLL路径
        let path = PathBuf::from(env!("LIB_PATH"));
        // 加载DLL
        unsafe { Library::new(path).expect("unload library") }
    });
}

type SingBoxStart =
unsafe extern "system" fn(configPathPtr: *mut ::std::os::raw::c_char) -> ::std::os::raw::c_int;

type SingBoxRefresh =
unsafe extern "system" fn(configPathPtr: *mut ::std::os::raw::c_char) -> ::std::os::raw::c_int;

type SingBoxStop = unsafe extern "system" fn() -> ::std::os::raw::c_int;

type SingBoxJsonToSrs = unsafe extern "system" fn(
    jsonPathPtr: *mut ::std::os::raw::c_char,
    srsPathPtr: *mut ::std::os::raw::c_char,
) -> ::std::os::raw::c_int;

unsafe fn get<T>(symbol: &[u8]) -> Symbol<T> {
    unsafe {
        lib.get()
            .expect("failed load lib")
            .get(symbol)
            .expect("failed get symbol")
    }
}

pub unsafe fn SingBoxStart(configPathPtr: *mut ::std::os::raw::c_char) -> ::std::os::raw::c_int {
    unsafe {
        let s: Symbol<SingBoxStart> = get(b"SingBoxStart\0");
        s(configPathPtr)
    }
}
pub unsafe fn SingBoxRefresh(configPathPtr: *mut ::std::os::raw::c_char) -> ::std::os::raw::c_int {
    unsafe {
        let s: Symbol<SingBoxRefresh> = get(b"SingBoxRefresh\0");
        s(configPathPtr)
    }
}
pub unsafe fn SingBoxStop() -> ::std::os::raw::c_int {
    unsafe {
        let s: Symbol<SingBoxStop> = get(b"SingBoxStop\0");
        s()
    }
}
pub unsafe fn SingBoxJsonToSrs(
    jsonPathPtr: *mut ::std::os::raw::c_char,
    srsPathPtr: *mut ::std::os::raw::c_char,
) -> ::std::os::raw::c_int {
    unsafe {
        let s: Symbol<SingBoxJsonToSrs> = get(b"SingBoxJsonToSrs\0");
        s(jsonPathPtr, srsPathPtr)
    }
}
