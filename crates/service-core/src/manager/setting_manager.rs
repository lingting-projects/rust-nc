use std::sync::Arc;

use anyhow::{Result, bail};
use lib_core::LogLevel;
use lib_db::DbContext;
use lib_system::startup;
use types_core::dto::setting::{Setting, SettingKernel, SettingRun, SettingSoftware};
use types_core::entity::setting::AppConfigKey;

use crate::service::app_config_service::AppConfigService;

/// 应用设置的业务入口。
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
    pub async fn get(&self) -> Result<Setting> {
        Ok(Setting {
            kernel: self.kernel().await?,
            software: self.software().await?,
            run: self.run().await?,
        })
    }

    /// 读取内核设置。
    pub async fn kernel(&self) -> Result<SettingKernel> {
        let keys = [
            AppConfigKey::SettingKernelSingBoxVersion,
            AppConfigKey::SettingKernelUiListen,
            AppConfigKey::SettingKernelUiPort,
            AppConfigKey::SettingKernelMixedListen,
            AppConfigKey::SettingKernelMixedPort,
            AppConfigKey::SettingKernelDnsCn,
            AppConfigKey::SettingKernelDnsProxy,
            AppConfigKey::SettingKernelLogLevel,
        ];
        let map = self.app_config.map(&keys).await?;
        let default = SettingKernel::default();

        Ok(SettingKernel {
            sing_box_version: text(&map, &keys[0], default.sing_box_version),
            ui_listen: text(&map, &keys[1], default.ui_listen),
            ui_port: number(&map, &keys[2]).unwrap_or(default.ui_port),
            mixed_listen: text(&map, &keys[3], default.mixed_listen),
            mixed_port: number(&map, &keys[4]).unwrap_or(default.mixed_port),
            dns_cn: list(&map, &keys[5]).unwrap_or(default.dns_cn),
            dns_proxy: list(&map, &keys[6]).unwrap_or(default.dns_proxy),
            log_level: level(&map, &keys[7]).unwrap_or(default.log_level),
        })
    }

    /// 读取软件设置。
    pub async fn software(&self) -> Result<SettingSoftware> {
        let keys = [AppConfigKey::SettingSoftwareMinimize];
        let map = self.app_config.map(&keys).await?;
        let default = SettingSoftware::default();

        Ok(SettingSoftware {
            startup: startup::is_startup()?,
            minimize: boolean(&map, &keys[0]).unwrap_or(default.minimize),
            version: default.version,
            fast_github: default.fast_github,
            test_url: default.test_url,
        })
    }

    /// 读取运行设置。
    pub async fn run(&self) -> Result<SettingRun> {
        let keys = [
            AppConfigKey::SettingRunAuto,
            AppConfigKey::SettingRunSelected,
        ];
        let map = self.app_config.map(&keys).await?;
        let default = SettingRun::default();

        Ok(SettingRun {
            auto: boolean(&map, &keys[0]).unwrap_or(default.auto),
            selected: selected(&map, &keys[1]),
        })
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
        self.app_config.set_all(&items).await
    }

    /// 写入内核日志等级。
    ///
    /// 与监听设置分开保存：日志等级只影响新生成的配置，
    /// 需要用户自行刷新配置并重启内核后才生效。
    pub async fn upsert_kernel_log_level(&self, log_level: LogLevel) -> Result<()> {
        self.app_config
            .set(&AppConfigKey::SettingKernelLogLevel, log_level.as_str())
            .await
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
        self.app_config.set_all(&items).await
    }

    /// 记录选中的配置 id。
    pub async fn select(&self, id: &str) -> Result<()> {
        self.app_config
            .set(&AppConfigKey::SettingRunSelected, id)
            .await
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

/// 取文本配置，缺失时使用默认值。
fn text(
    map: &std::collections::HashMap<String, String>,
    key: &AppConfigKey,
    default: String,
) -> String {
    map.get(&key.to_string()).cloned().unwrap_or(default)
}

/// 取数字配置，缺失或非法时返回 `None`。
fn number<T>(map: &std::collections::HashMap<String, String>, key: &AppConfigKey) -> Option<T>
where
    T: std::str::FromStr,
{
    map.get(&key.to_string())?.parse().ok()
}

/// 取日志等级配置，缺失或非法时返回 `None`，由调用方退回默认等级。
fn level(map: &std::collections::HashMap<String, String>, key: &AppConfigKey) -> Option<LogLevel> {
    map.get(&key.to_string())?.parse().ok()
}

/// 取布尔配置，缺失或非法时返回 `None`。
fn boolean(map: &std::collections::HashMap<String, String>, key: &AppConfigKey) -> Option<bool> {
    map.get(&key.to_string())?.parse().ok()
}

/// 取字符串数组配置，缺失或非法时返回 `None`。
fn list(
    map: &std::collections::HashMap<String, String>,
    key: &AppConfigKey,
) -> Option<Vec<String>> {
    serde_json::from_str(map.get(&key.to_string())?).ok()
}

/// 取选中的配置 id，空串视为未选中。
fn selected(map: &std::collections::HashMap<String, String>, key: &AppConfigKey) -> Option<String> {
    map.get(&key.to_string())
        .filter(|value| !value.is_empty())
        .cloned()
}
