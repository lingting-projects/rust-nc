use crate::core::AnyResult;
use crate::file;
use std::fs::File;
use std::path::Path;
use std::sync::{LazyLock, Mutex};

static out: LazyLock<Mutex<Option<File>>> = LazyLock::new(||Mutex::new(None));
static err: LazyLock<Mutex<Option<File>>> = LazyLock::new(||Mutex::new(None));

pub fn redirect_dir<P: AsRef<Path>>(_d: P) -> AnyResult<()> {
    let dir = _d.as_ref();
    let p_out = dir.join("out.log");
    let p_err = dir.join("err.log");
    file::create(&p_out)?;
    file::create(&p_err)?;

    let stdout_file = File::create(p_out)?;
    let stderr_file = File::create(p_err)?;

    #[cfg(not(target_os = "windows"))]
    {
        use std::os::unix::io::AsRawFd;
        unsafe {
            libc::dup2(stdout_file.as_raw_fd(), libc::STDOUT_FILENO);
            libc::dup2(stderr_file.as_raw_fd(), libc::STDERR_FILENO);
        }
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::io::AsRawHandle;
        use winapi::um::handleapi::INVALID_HANDLE_VALUE;
        use winapi::um::processenv::SetStdHandle;
        use winapi::um::winbase::{STD_ERROR_HANDLE, STD_OUTPUT_HANDLE};
        use winapi::um::winnt::HANDLE;

        unsafe {
            let out_handle = stdout_file.as_raw_handle() as HANDLE;
            let err_handle = stderr_file.as_raw_handle() as HANDLE;

            if out_handle != INVALID_HANDLE_VALUE {
                SetStdHandle(STD_OUTPUT_HANDLE, out_handle);
            }

            if err_handle != INVALID_HANDLE_VALUE {
                SetStdHandle(STD_ERROR_HANDLE, err_handle);
            }
        }
    }

    *out.lock().unwrap() = Some(stdout_file);
    *err.lock().unwrap() = Some(stderr_file);

    Ok(())
}
