use framework_proc_auto::{auto_enum, auto_type};

/// 应用配置键, 与 app_config 表已存在的 key 保持一致
#[auto_enum]
pub enum AppConfigKey {
    /// 数据库结构版本
    #[strum(serialize = "version")]
    #[serde(rename = "version")]
    Version,
    /// 运行: 是否自动启动
    #[strum(serialize = "setting:run:auto")]
    #[serde(rename = "setting:run:auto")]
    SettingRunAuto,
    /// 运行: 选中的配置 id
    #[strum(serialize = "setting:run:selected")]
    #[serde(rename = "setting:run:selected")]
    SettingRunSelected,
    /// 软件: 是否最小化启动
    #[strum(serialize = "setting:software:minimize")]
    #[serde(rename = "setting:software:minimize")]
    SettingSoftwareMinimize,
    /// 内核: sing-box 版本
    #[strum(serialize = "setting:kernel:sing_box_version")]
    #[serde(rename = "setting:kernel:sing_box_version")]
    SettingKernelSingBoxVersion,
    /// 内核: 面板监听地址
    #[strum(serialize = "setting:kernel:ui_listen")]
    #[serde(rename = "setting:kernel:ui_listen")]
    SettingKernelUiListen,
    /// 内核: 面板监听端口
    #[strum(serialize = "setting:kernel:ui_port")]
    #[serde(rename = "setting:kernel:ui_port")]
    SettingKernelUiPort,
    /// 内核: 混合代理监听地址
    #[strum(serialize = "setting:kernel:mixed_listen")]
    #[serde(rename = "setting:kernel:mixed_listen")]
    SettingKernelMixedListen,
    /// 内核: 混合代理端口
    #[strum(serialize = "setting:kernel:mixed_port")]
    #[serde(rename = "setting:kernel:mixed_port")]
    SettingKernelMixedPort,
    /// 内核: 国内 dns
    #[strum(serialize = "setting:kernel:dns_cn")]
    #[serde(rename = "setting:kernel:dns_cn")]
    SettingKernelDnsCn,
    /// 内核: 代理 dns
    #[strum(serialize = "setting:kernel:dns_proxy")]
    #[serde(rename = "setting:kernel:dns_proxy")]
    SettingKernelDnsProxy,
    /// 内核: 日志等级
    #[strum(serialize = "setting:kernel:log_level")]
    #[serde(rename = "setting:kernel:log_level")]
    SettingKernelLogLevel,
}

/// 应用配置, 对应 app_config 表。
///
/// `key` 取值见 [`AppConfigKey`]。
#[auto_type(default = false)]
pub struct AppConfig {
    pub key: String,
    pub value: String,
}
