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
    let parent = p.parent().unwrap();
    create_dir(parent)
}

pub fn create<P: AsRef<Path>>(path: P) -> AnyResult<File> {
    let p = path.as_ref();
    create_parent(p)?;
    let f = if p.exists() {
        File::open(p)?
    } else {
        File::create(p)?
    };

    Ok(f)
}

pub fn write<P: AsRef<Path>>(path: P, content: &str) -> AnyResult<()> {
    let mut file = create(path)?;
    file.write_all(content.as_bytes())?;
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
