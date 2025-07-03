use std::sync::Once;
use tklog::{Format, LEVEL, LOG};

static INIT: Once = Once::new();

pub fn init() {
    INIT.call_once(|| {
        // 配置 tklog
        LOG.set_console(true)
            // Sets the log level; default is Debug
            .set_level(LEVEL::Info)
            // Defines structured log output with chosen details
            .set_format(Format::LevelFlag | Format::Time | Format::ShortFileName)
            // Cuts logs by file size (1 MB), keeps 10 backups, compresses backups
            // .set_cutmode_by_size("tklogsize.txt", 1 << 20, 10, true)
            // Customizes log output format; default is "{level}{time} {file}:{message}"
            .set_formatter("{level}{time} {file}:{message}\n");

        log::info!("日志系统初始化完成");
    });
}
