use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub fn write_to<P: AsRef<Path>>(path: P, content: &str) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

pub fn delete<P: AsRef<Path>>(path: P) -> std::io::Result<()> {
    match fs::remove_file(path) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}
