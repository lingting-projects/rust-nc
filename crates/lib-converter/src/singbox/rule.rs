use serde::Serialize;

use crate::constants::TAG_HTTP_CLIENT;
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
    /// 下载规则集使用的 http 客户端, 1.14 起取代 `download_detour`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_client: Option<String>,
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
                http_client: Some(TAG_HTTP_CLIENT.to_string()),
                update_interval: Some("1d".to_string()),
            };
        }

        SingBoxRule {
            tag: tag.to_string(),
            type_: "local".to_string(),
            format,
            url: None,
            path: Some(self.path.clone()),
            http_client: None,
            update_interval: None,
        }
    }
}
