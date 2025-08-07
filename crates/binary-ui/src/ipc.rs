use interprocess::{
    local_socket,
    local_socket::{
        traits::{Listener, Stream}, GenericFilePath, ListenerOptions, Name,
        ToFsName,
    },
};
use library_core::core::AnyResult;
use std::io::Write;

#[cfg(target_os = "windows")]
const prefix: &str = r"\\.\pipe\";

#[cfg(target_os = "windows")]
fn to_name(path: &str) -> AnyResult<Name> {
    let p = if path.starts_with(prefix) {
        path.to_string()
    } else {
        format!("{}{}", prefix, path)
    };
    let name = p.to_fs_name::<GenericFilePath>()?;
    Ok(name)
}

#[cfg(not(target_os = "windows"))]
fn to_name(path: &str) -> AnyResult<Name> {
    let name = path.to_fs_name::<GenericFilePath>()?;
    Ok(name)
}

pub struct IpcStream {
    inner: local_socket::Stream,
}

impl IpcStream {
    pub fn new(path: &str) -> AnyResult<Self> {
        let name = to_name(path)?;
        let stream = local_socket::Stream::connect(name)?;
        Ok(Self { inner: stream })
    }

    pub fn write(&mut self, content: &str) -> AnyResult<()> {
        let bytes = content.as_bytes();
        self.inner.write(bytes)?;
        Ok(())
    }
}

pub struct IpcServer {
    inner: local_socket::Listener,
}

impl IpcServer {
    pub fn new(path: &str) -> AnyResult<Self> {
        let name = to_name(path)?;
        let listener = ListenerOptions::new()
            .name(name)
            .create_sync()?;

        Ok(Self { inner: listener })
    }

    pub fn next(&self) -> AnyResult<IpcStream> {
        let stream = self.inner.accept()?;
        Ok(IpcStream { inner: stream })
    }
}
