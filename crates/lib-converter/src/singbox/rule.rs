//! sing-box 规则集。

use serde::Serialize;

use crate::constants::OUT_DIRECT;
use crate::rule::Rule;

/// sing-box 规则集。
#[derive(Debug, Clone, Serialize)]
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

impl Rule {
    /// 转换为 sing-box 规则集。
    pub(crate) fn sing_box(&self, tag: &str) -> SingBoxRule {
        let format = if self.path.ends_with("srs") {
            "binary"
        } else {
            "source"
        }
        .to_string();

        if self.remote {
            return SingBoxRule {
                tag: tag.to_string(),
                type_: "remote".to_string(),
                format,
                url: Some(self.path.clone()),
                path: None,
                download_detour: Some(OUT_DIRECT.to_string()),
                update_interval: Some("1d".to_string()),
            };
        }

        SingBoxRule {
            tag: tag.to_string(),
            type_: "local".to_string(),
            format,
            url: None,
            path: Some(self.path.clone()),
            download_detour: None,
            update_interval: None,
        }
    }
}
