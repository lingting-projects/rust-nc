use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_int};
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

type SingBoxRunning = unsafe extern "system" fn() -> c_int;

type SingBoxStart =
    unsafe extern "system" fn(configPathPtr: *mut c_char, workDirPtr: *mut c_char) -> c_int;

type SingBoxStop = unsafe extern "system" fn() -> c_int;

type SingBoxJsonToSrs =
    unsafe extern "system" fn(jsonPathPtr: *mut c_char, srsPathPtr: *mut c_char) -> c_int;

fn get<T>(symbol: &[u8]) -> Symbol<T> {
    unsafe {
        lib.get()
            .expect("failed load lib")
            .get(symbol)
            .expect("failed get symbol")
    }
}

pub fn SingBoxRunning() -> c_int {
    unsafe {
        let s: Symbol<SingBoxRunning> = get(b"SingBoxRunning\0");
        s()
    }
}
pub fn SingBoxStart(configPathPtr: *mut c_char, workDirPtr: *mut c_char) -> c_int {
    unsafe {
        let s: Symbol<SingBoxStart> = get(b"SingBoxStart\0");
        s(configPathPtr, workDirPtr)
    }
}
pub fn SingBoxStop() -> c_int {
    unsafe {
        let s: Symbol<SingBoxStop> = get(b"SingBoxStop\0");
        s()
    }
}
pub fn SingBoxJsonToSrs(jsonPathPtr: *mut c_char, srsPathPtr: *mut c_char) -> c_int {
    unsafe {
        let s: Symbol<SingBoxJsonToSrs> = get(b"SingBoxJsonToSrs\0");
        s(jsonPathPtr, srsPathPtr)
    }
}
