use crate::core::fast;
use crate::kernel::key_direct;
use serde::Serialize;

pub enum RuleType {
    // 针对ip的规则
    Ip,
    // 针对进程的规则
    Process,
    // 其他规则
    Other,
}

pub struct Rule {
    pub path: String,
    pub rule_type: RuleType,
    pub remote: bool,
}

impl Rule {
    pub fn from_local(rule_type: RuleType, path: String) -> Self {
        Self {
            rule_type,
            path,
            remote: false,
        }
    }

    pub fn from_remote(rule_type: RuleType, url: String) -> Self {
        let path = fast(&url);
        Self {
            rule_type,
            path,
            remote: true,
        }
    }

    pub fn sing_box(&self, tag: &str) -> SingBoxRule {
        let tag = tag.into();
        let format = if self.path.ends_with("srs") {
            "binary"
        } else {
            "source"
        }
        .into();

        if self.remote {
            SingBoxRule {
                tag,
                type_: "remote".into(),
                format,
                url: Some(self.path.to_string()),
                path: None,
                download_detour: Some(key_direct.into()),
                update_interval: Some("1d".into()),
            }
        } else {
            SingBoxRule {
                tag,
                type_: "local".into(),
                format,
                url: None,
                path: Some(self.path.to_string()),
                download_detour: Some(key_direct.into()),
                update_interval: Some("1d".into()),
            }
        }
    }

    pub fn clash(&self, tag: &str) -> ClashRule {
        let name = tag.into();
        let format = "yaml".into();
        let behavior = "classical".into();

        if self.remote {
            ClashRule {
                name,
                format,
                behavior,
                type_: "http".into(),
                url: Some(self.path.to_string()),
                path: format!("./rules/{}.yml", tag),
                interval: Some(86400),
            }
        } else {
            ClashRule {
                name,
                format,
                behavior,
                type_: "file".into(),
                url: None,
                path: self.path.to_string(),
                interval: None,
            }
        }
    }
}

#[derive(Serialize)]
pub struct SingBoxRule {
    pub tag: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_detour: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_interval: Option<String>,
}

#[derive(Serialize)]
pub struct ClashRule {
    #[serde(skip)]
    pub name: String,
    pub format: String,
    pub behavior: String,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<u32>,
}
