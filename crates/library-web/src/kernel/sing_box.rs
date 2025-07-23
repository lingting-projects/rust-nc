use library_core::core::AnyResult;
use std::path::Path;

pub fn json_srs<P: AsRef<Path>>(json: P, srs: P) -> AnyResult<()> {
    // json 配置 转 srs 配置
    Ok(())
}
