use crate::core::AnyResult;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub fn create<P: AsRef<Path>>(path: P) -> AnyResult<File> {
    let p = path.as_ref();
    let parent = p.parent().unwrap();
    if !parent.exists() {
        fs::create_dir_all(parent)?;
    }
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
