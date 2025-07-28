use crate::route_global::to_value;
use library_core::app_config::AppConfig;
use library_core::boolean::is_true;
use library_core::core::AnyResult;
use library_core::sqlite::execute;
use library_nc::core::FAST_GItHUB_PREFIX;
use library_nc::kernel::{
    default_mixed_listen, default_mixed_port, default_ui, dns_default_cn, dns_default_proxy,
    test_url,
};
use serde::{Deserialize, Serialize};
use sqlite::Value;
use std::clone::Clone;
use std::convert::Into;
use std::string::ToString;
use std::sync::LazyLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TblSetting {
    pub kernel: TblSettingKernel,
    pub software: TblSettingSoftware,
    pub run: TblSettingRun,
}

impl TblSetting {
    pub fn get() -> AnyResult<Self> {
        Ok(TblSetting {
            kernel: TblSettingKernel::get()?,
            software: TblSettingSoftware::get()?,
            run: TblSettingRun::get()?,
        })
    }

    pub fn upsert(&self) -> AnyResult<i32> {
        let mut sql = format!(
            "REPLACE INTO {} (`key`, `value`) VALUES ",
            AppConfig::table_name
        );
        let mut sets = Vec::new();
        let mut args = Vec::new();
        self.run.append_upsert(&mut sets, &mut args);
        self.software.append_upsert(&mut sets, &mut args);
        self.kernel.append_upsert(&mut sets, &mut args)?;

        sql.push_str(&sets.join(","));

        execute(&sql, args)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TblSettingRun {
    pub auto: bool,
    pub selected: Option<String>,
}

impl TblSettingRun {
    pub const default: LazyLock<TblSettingRun> = LazyLock::new(|| TblSettingRun {
        auto: false,
        selected: None,
    });

    pub const key_auto: &'static str = "setting:run:auto";
    pub const key_selected: &'static str = "setting:run:selected";

    pub fn get() -> AnyResult<Self> {
        let map = AppConfig::keys(vec![Self::key_auto, Self::key_selected])?;
        let run = TblSettingRun {
            auto: map
                .get(Self::key_auto)
                .map(|v| is_true(v))
                .unwrap_or(Self::default.auto),
            selected: map.get(Self::key_selected).map(|v| v.to_string()),
        };

        Ok(run)
    }

    pub fn append_upsert(&self, sets: &mut Vec<String>, args: &mut Vec<Value>) {
        sets.push("(?,?)".to_string());
        args.push(Value::from(Self::key_auto));
        args.push(to_value(self.auto));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TblSettingSoftware {
    pub startup: bool,
    pub minimize: bool,
    pub version: String,
    pub fast_github: String,
    pub test_url: String,
}

impl TblSettingSoftware {
    pub const default: LazyLock<TblSettingSoftware> = LazyLock::new(|| TblSettingSoftware {
        startup: false,
        minimize: false,
        version: "0.0.0".to_string(),
        fast_github: FAST_GItHUB_PREFIX.clone(),
        test_url: test_url.into(),
    });

    pub const key_minimize: &'static str = "setting:software:minimize";
    pub const key_version: &'static str = AppConfig::key_version;

    pub fn get() -> AnyResult<Self> {
        let map = AppConfig::keys(vec![Self::key_minimize, Self::key_version])?;
        let software = TblSettingSoftware {
            startup: false,
            minimize: map
                .get(Self::key_minimize)
                .map(|v| is_true(v))
                .unwrap_or(Self::default.minimize),
            version: map
                .get(Self::key_version)
                .map(|v| v.clone())
                .unwrap_or(Self::default.version.clone()),
            fast_github: Self::default.fast_github.clone(),
            test_url: Self::default.test_url.clone(),
        };

        Ok(software)
    }

    pub fn is_minimize() -> bool {
        AppConfig::get(Self::key_minimize)
            .ok()
            .flatten()
            .map(|v| is_true(&v))
            .unwrap_or(Self::default.minimize)
    }

    pub fn append_upsert(&self, sets: &mut Vec<String>, args: &mut Vec<Value>) {
        sets.push("(?,?)".to_string());
        args.push(Value::from(Self::key_minimize));
        args.push(to_value(self.minimize));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

    pub fn get() -> AnyResult<Self> {
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

    pub fn append_upsert(&self, sets: &mut Vec<String>, args: &mut Vec<Value>) -> AnyResult<()> {
        sets.push("(?,?)".to_string());
        args.push(Value::from(Self::key_ui));
        args.push(Value::String(self.ui.clone()));
        sets.push("(?,?)".to_string());
        args.push(Value::from(Self::key_mixed_listen));
        args.push(Value::String(self.mixed_listen.clone()));
        sets.push("(?,?)".to_string());
        args.push(Value::from(Self::key_mixed_port));
        args.push(Value::Integer(self.mixed_port as i64));
        sets.push("(?,?)".to_string());
        args.push(Value::from(Self::key_dns_cn));
        args.push(Value::String(serde_json::to_string(
            &serde_json::Value::from(self.dns_cn.clone()),
        )?));
        sets.push("(?,?)".to_string());
        args.push(Value::from(Self::key_dns_proxy));
        args.push(Value::String(serde_json::to_string(
            &serde_json::Value::from(self.dns_proxy.clone()),
        )?));
        Ok(())
    }
}
