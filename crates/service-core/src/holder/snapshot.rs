//! 设置的内存快照。
//!
//! 快照保存 `app_config` 的全部键值, 以及读取时无法由数据库得到的系统状态;
//! 需要具体设置时再由快照转换为对应的 DTO, 不再访问数据库。

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use lib_core::LogLevel;
use lib_db::DbContext;
use lib_system::startup;
use types_core::dto::setting::{Setting, SettingKernel, SettingRun, SettingSoftware};
use types_core::entity::setting::AppConfigKey;

use crate::service::app_config_service::AppConfigService;

/// 一份完整的内存设置。
pub(crate) struct Snapshot {
    /// `app_config` 的全部键值
    config: HashMap<String, String>,
    /// 是否已注册开机自启, 取自系统计划任务状态
    startup: bool,
}

impl Snapshot {
    /// 读取全部设置。
    pub(crate) async fn load(context: &Arc<DbContext>) -> Result<Self> {
        let config = AppConfigService::new(context.clone()).all().await?;
        // 查询计划任务会启动系统进程, 放到阻塞线程池, 避免占住异步线程
        let startup = tokio::task::spawn_blocking(startup::is_startup).await??;

        Ok(Self { config, startup })
    }

    /// 全部设置。
    pub(crate) fn all(&self) -> Setting {
        Setting {
            kernel: self.kernel(),
            software: self.software(),
            run: self.run(),
        }
    }

    /// 内核设置。
    pub(crate) fn kernel(&self) -> SettingKernel {
        let keys = [
            AppConfigKey::SettingKernelSingBoxVersion,
            AppConfigKey::SettingKernelUiListen,
            AppConfigKey::SettingKernelUiPort,
            AppConfigKey::SettingKernelMixedListen,
            AppConfigKey::SettingKernelMixedPort,
            AppConfigKey::SettingKernelDnsCn,
            AppConfigKey::SettingKernelDnsProxy,
            AppConfigKey::SettingKernelLogLevel,
            AppConfigKey::SettingKernelAutoRun,
        ];
        let default = SettingKernel::default();

        SettingKernel {
            sing_box_version: self.text(&keys[0], default.sing_box_version),
            ui_listen: self.text(&keys[1], default.ui_listen),
            ui_port: self.number(&keys[2]).unwrap_or(default.ui_port),
            mixed_listen: self.text(&keys[3], default.mixed_listen),
            mixed_port: self.number(&keys[4]).unwrap_or(default.mixed_port),
            dns_cn: self.list(&keys[5]).unwrap_or(default.dns_cn),
            dns_proxy: self.list(&keys[6]).unwrap_or(default.dns_proxy),
            log_level: self.level(&keys[7]).unwrap_or(default.log_level),
            auto_run: self.boolean(&keys[8]).unwrap_or(default.auto_run),
        }
    }

    /// 软件设置。
    pub(crate) fn software(&self) -> SettingSoftware {
        let default = SettingSoftware::default();

        SettingSoftware {
            startup: self.startup,
            minimize: self
                .boolean(&AppConfigKey::SettingSoftwareMinimize)
                .unwrap_or(default.minimize),
            tray: self
                .boolean(&AppConfigKey::SettingSoftwareTray)
                .unwrap_or(default.tray),
            version: default.version,
            fast_github: default.fast_github,
            test_url: default.test_url,
        }
    }

    /// 运行设置。
    pub(crate) fn run(&self) -> SettingRun {
        let default = SettingRun::default();

        SettingRun {
            auto: self
                .boolean(&AppConfigKey::SettingRunAuto)
                .unwrap_or(default.auto),
            selected: self.selected(&AppConfigKey::SettingRunSelected),
        }
    }

    /// 取文本配置, 缺失时使用默认值。
    fn text(&self, key: &AppConfigKey, default: String) -> String {
        self.value(key).map(str::to_string).unwrap_or(default)
    }

    /// 取数字配置, 缺失或非法时返回 `None`。
    fn number<T: std::str::FromStr>(&self, key: &AppConfigKey) -> Option<T> {
        self.value(key)?.parse().ok()
    }

    /// 取日志等级配置, 缺失或非法时返回 `None`, 由调用方退回默认等级。
    fn level(&self, key: &AppConfigKey) -> Option<LogLevel> {
        self.value(key)?.parse().ok()
    }

    /// 取布尔配置, 缺失或非法时返回 `None`。
    fn boolean(&self, key: &AppConfigKey) -> Option<bool> {
        self.value(key)?.parse().ok()
    }

    /// 取字符串数组配置, 缺失或非法时返回 `None`。
    fn list(&self, key: &AppConfigKey) -> Option<Vec<String>> {
        serde_json::from_str(self.value(key)?).ok()
    }

    /// 取选中的配置 id, 空串视为未选中。
    fn selected(&self, key: &AppConfigKey) -> Option<String> {
        self.value(key)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    }

    /// 按配置键取原始文本。
    fn value(&self, key: &AppConfigKey) -> Option<&str> {
        self.config.get(&key.to_string()).map(String::as_str)
    }
}

impl Default for Snapshot {
    /// 空快照: 所有设置都退回默认值。
    fn default() -> Self {
        Self {
            config: HashMap::new(),
            startup: false,
        }
    }
}
