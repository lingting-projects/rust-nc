use anyhow::Result;
use framework_startup::{Startup, StartupConfig};

/// 自启名称，Windows 下同时是计划任务名。
const NAME: &str = "LingtingNcStartup";

/// 是否已注册开机自启。
pub fn is_startup() -> Result<bool> {
    startup()?.exists()
}

/// 注册开机自启。
///
/// 使用用户级自启：随当前用户登录启动。Windows 下要求最高可用权限，
/// 与 exe 内嵌的 `requireAdministrator` 清单配合，使进程启动后即具备管理员权限。
/// 已注册时强制覆盖，让可执行文件路径、提权级别等配置变更在下次登录时生效。
pub fn enable() -> Result<()> {
    startup()?.install_force()
}

/// 取消开机自启；未注册时直接返回。
pub fn disable() -> Result<()> {
    startup()?.uninstall()
}

/// 当前可执行文件的自启配置。
fn startup() -> Result<Startup> {
    let mut config = StartupConfig::new(NAME);
    config.description = "lingting network control".to_string();
    config.working_directory = std::env::current_exe()?
        .parent()
        .map(std::path::Path::to_path_buf);
    #[cfg(target_os = "windows")]
    {
        config.highest = true;
    }

    Startup::user(config)
}
