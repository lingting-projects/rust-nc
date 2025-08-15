use crate::core::AnyResult;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub fn create_dir<P: AsRef<Path>>(path: P) -> AnyResult<()> {
    let p = path.as_ref();
    if !p.exists() {
        fs::create_dir_all(p)?;
    }
    Ok(())
}

pub fn create_parent<P: AsRef<Path>>(path: P) -> AnyResult<()> {
    let p = path.as_ref();
    if !p.exists() {
        let parent = p.parent().unwrap();
        create_dir(parent)?;
    }
    Ok(())
}

pub fn create<P: AsRef<Path>>(path: P) -> AnyResult<()> {
    let p = path.as_ref();
    if !p.exists() {
        create_parent(p)?;
        File::create(p)?;
    }

    Ok(())
}

pub fn overwrite<P: AsRef<Path>>(path: P, content: &str) -> AnyResult<()> {
    create(&path)?;
    let bytes = content.as_bytes();
    overwrite_bytes(path, bytes)
}

pub fn overwrite_bytes<P: AsRef<Path>>(path: P, bytes: &[u8]) -> AnyResult<()> {
    create(&path)?;
    let mut file = File::options().write(true).truncate(true).open(path)?;
    file.write_all(bytes)?;
    Ok(())
}

pub fn copy<P: AsRef<Path>>(source: P, target: P) -> AnyResult<()> {
    let t = target.as_ref();
    if t.exists() {
        return Ok(());
    }
    copy_force(source, target)
}

pub fn copy_force<P: AsRef<Path>>(source: P, target: P) -> AnyResult<()> {
    create_parent(target.as_ref())?;
    fs::copy(source, target)?;
    Ok(())
}

pub fn delete_dir<P: AsRef<Path>>(path: P) -> std::io::Result<()> {
    match fs::remove_dir_all(path) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

pub fn delete<P: AsRef<Path>>(path: P) -> std::io::Result<()> {
    match fs::remove_file(path) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}
