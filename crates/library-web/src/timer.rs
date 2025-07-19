use library_nc::core::AnyResult;

pub fn init() -> AnyResult<()> {
    log::debug!("初始化定时器");
    Ok(())
}
