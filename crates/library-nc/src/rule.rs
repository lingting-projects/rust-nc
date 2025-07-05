use crate::core::fast;
use std::collections::HashMap;

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

    pub fn sing_box(&self, tag: &str) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("tag", tag);

        if self.remote {
            map.insert("url", &self.path);
            map.insert("type", "remote");
            map.insert("download_detour", "DIRECT");
            map.insert("update_interval", "1d");
        } else {
            map.insert("path", &self.path);
            map.insert("type", "local");
        }

        map.insert(
            "format",
            if self.path.ends_with("srs") {
                "binary"
            } else {
                "source"
            },
        );

        map.into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }
}
