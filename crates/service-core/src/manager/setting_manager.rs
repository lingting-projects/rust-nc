use std::sync::Arc;

use anyhow::{Result, bail};
use lib_core::LogLevel;
use lib_db::DbContext;
use lib_system::startup;
use types_core::dto::setting::{Setting, SettingKernel, SettingRun, SettingSoftware};
use types_core::entity::setting::AppConfigKey;

use crate::holder::settings::SettingsHolder;
use crate::service::app_config_service::AppConfigService;

/// 应用设置的业务入口。
///
/// 读取全部走 [`SettingsHolder`] 的内存快照; 写入成功后唤醒它重新加载,
/// 保证后续从内存读到的就是刚写入的设置。
pub struct SettingManager {
    app_config: AppConfigService,
    /// 开机自启所需的中间文件目录
    cache_dir: std::path::PathBuf,
}

impl SettingManager {
    /// 绑定请求级数据库上下文与缓存目录。
    pub fn new(context: Arc<DbContext>, cache_dir: impl Into<std::path::PathBuf>) -> Self {
        Self {
            app_config: AppConfigService::new(context),
            cache_dir: cache_dir.into(),
        }
    }

    /// 读取全部设置。
    pub fn get(&self) -> Setting {
        SettingsHolder::all()
    }

    /// 读取内核设置。
    pub fn kernel(&self) -> SettingKernel {
        SettingsHolder::kernel()
    }

    /// 读取软件设置。
    pub fn software(&self) -> SettingSoftware {
        SettingsHolder::software()
    }

    /// 读取运行设置。
    pub fn run(&self) -> SettingRun {
        SettingsHolder::run()
    }

    /// 只写入内核监听设置，其余设置保持不变。
    ///
    /// 设置页面的内核分区单独保存，避免为了改监听端口而读取并回写整份设置。
    pub async fn upsert_kernel_listen(
        &self,
        ui_listen: &str,
        ui_port: u16,
        mixed_listen: &str,
        mixed_port: u16,
    ) -> Result<()> {
        let items = [
            (AppConfigKey::SettingKernelUiListen, ui_listen.to_string()),
            (AppConfigKey::SettingKernelUiPort, ui_port.to_string()),
            (
                AppConfigKey::SettingKernelMixedListen,
                mixed_listen.to_string(),
            ),
            (AppConfigKey::SettingKernelMixedPort, mixed_port.to_string()),
        ];
        self.app_config.set_all(&items).await?;
        SettingsHolder::wake_and_wait().await;
        Ok(())
    }

    /// 写入内核日志等级。
    ///
    /// 与监听设置分开保存：日志等级只影响新生成的配置，
    /// 需要用户自行刷新配置并重启内核后才生效。
    pub async fn upsert_kernel_log_level(&self, log_level: LogLevel) -> Result<()> {
        self.app_config
            .set(&AppConfigKey::SettingKernelLogLevel, log_level.as_str())
            .await?;
        SettingsHolder::wake_and_wait().await;
        Ok(())
    }

    /// 写入设置，先同步开机自启状态再落库。
    pub async fn upsert(&self, setting: &Setting) -> Result<()> {
        self.sync_startup(setting.software.startup).await?;

        let mut items = Vec::new();
        items.push((AppConfigKey::SettingRunAuto, setting.run.auto.to_string()));
        items.push((
            AppConfigKey::SettingSoftwareMinimize,
            setting.software.minimize.to_string(),
        ));
        items.push((
            AppConfigKey::SettingKernelUiListen,
            setting.kernel.ui_listen.clone(),
        ));
        items.push((
            AppConfigKey::SettingKernelUiPort,
            setting.kernel.ui_port.to_string(),
        ));
        items.push((
            AppConfigKey::SettingKernelMixedListen,
            setting.kernel.mixed_listen.clone(),
        ));
        items.push((
            AppConfigKey::SettingKernelMixedPort,
            setting.kernel.mixed_port.to_string(),
        ));
        items.push((
            AppConfigKey::SettingKernelDnsCn,
            serde_json::to_string(&setting.kernel.dns_cn)?,
        ));
        items.push((
            AppConfigKey::SettingKernelDnsProxy,
            serde_json::to_string(&setting.kernel.dns_proxy)?,
        ));
        items.push((
            AppConfigKey::SettingKernelLogLevel,
            setting.kernel.log_level.as_str().to_string(),
        ));
        self.app_config.set_all(&items).await?;
        SettingsHolder::wake_and_wait().await;
        Ok(())
    }

    /// 记录选中的配置 id。
    pub async fn select(&self, id: &str) -> Result<()> {
        self.app_config
            .set(&AppConfigKey::SettingRunSelected, id)
            .await?;
        SettingsHolder::wake_and_wait().await;
        Ok(())
    }

    /// 同步开机自启状态，操作失败时保持数据库不变。
    async fn sync_startup(&self, enabled: bool) -> Result<()> {
        if enabled {
            if startup::is_startup()? {
                return Ok(());
            }
            if !startup::enable(&self.cache_dir)? {
                bail!("设置开机自启失败");
            }
            return Ok(());
        }

        if !startup::disable()? {
            bail!("关闭开机自启失败");
        }
        Ok(())
    }
}
