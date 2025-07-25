use library_core::app_config::AppConfig;
use library_core::core::AnyResult;
use library_nc::kernel::{
    default_mixed_listen, default_mixed_port, default_ui, dns_default_cn, dns_default_proxy,
};
use serde::{Deserialize, Serialize};
use std::clone::Clone;
use std::convert::Into;
use std::string::ToString;
use std::sync::LazyLock;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct TblSetting {}

impl TblSetting {}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct TblSettingKernel {
    pub sing_box_version: String,
    pub ui: String,
    pub mixed_listen: String,
    pub mixed_port: u16,
    pub dns_cn: Vec<String>,
    pub dns_proxy: Vec<String>,
}

impl TblSettingKernel {
    pub const default: LazyLock<TblSettingKernel> = LazyLock::new(|| TblSettingKernel {
        sing_box_version: "1.11.9".into(),
        ui: default_ui.to_string(),
        mixed_listen: default_mixed_listen.to_string(),
        mixed_port: default_mixed_port,
        dns_cn: dns_default_cn.clone(),
        dns_proxy: dns_default_proxy.clone(),
    });

    pub const key_sing_box_version: &'static str = "setting:kernel:sing_box_version";
    pub const key_ui: &'static str = "setting:kernel:ui";
    pub const key_mixed_listen: &'static str = "setting:kernel:mixed_listen";
    pub const key_mixed_port: &'static str = "setting:kernel:mixed_port";
    pub const key_dns_cn: &'static str = "setting:kernel:dns_cn";
    pub const key_dns_proxy: &'static str = "setting:kernel:dns_proxy";

    pub fn new() -> AnyResult<Self> {
        let map = AppConfig::keys(vec![
            Self::key_sing_box_version,
            Self::key_ui,
            Self::key_mixed_listen,
            Self::key_mixed_port,
            Self::key_dns_cn,
            Self::key_dns_proxy,
        ])?;

        let kernel = Self {
            sing_box_version: map
                .get(Self::key_sing_box_version)
                .map(|v| v.to_string())
                .unwrap_or(Self::default.sing_box_version.clone()),
            ui: map
                .get(Self::key_ui)
                .map(|v| v.to_string())
                .unwrap_or(Self::default.ui.clone()),
            mixed_listen: map
                .get(Self::key_mixed_listen)
                .map(|v| v.to_string())
                .unwrap_or(Self::default.mixed_listen.clone()),
            mixed_port: map
                .get(Self::key_mixed_port)
                .map(|v| v.parse::<u16>().ok())
                .flatten()
                .unwrap_or(Self::default.mixed_port),
            dns_cn: map
                .get(Self::key_dns_cn)
                .map(|v| serde_json::from_str(v).ok())
                .flatten()
                .unwrap_or(Self::default.dns_cn.clone()),
            dns_proxy: map
                .get(Self::key_dns_proxy)
                .map(|v| serde_json::from_str(v).ok())
                .flatten()
                .unwrap_or(Self::default.dns_proxy.clone()),
        };
        Ok(kernel)
    }
}
