use std::collections::HashMap;
use std::fmt;
use std::sync::OnceLock;

use regex::Regex;

/// 区域信息。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Area {
    pub code: String,
    pub name_cn: String,
    pub name_en: String,
    pub name_local: String,
}

impl fmt::Display for Area {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.name_cn)
    }
}

struct Data {
    all: Vec<Area>,
    /// 与 `all` 一一对应的区域码匹配正则
    patterns: Vec<Regex>,
}

static DATA: OnceLock<Data> = OnceLock::new();

fn data() -> &'static Data {
    DATA.get_or_init(|| {
        let json = include_str!("../../../assets/countries.json");
        let root: HashMap<String, HashMap<String, String>> =
            serde_json::from_str(json).expect("解析 countries.json 失败");

        let all: Vec<Area> = root
            .into_iter()
            .map(|(code, value)| Area {
                code,
                name_cn: value.get("name").cloned().unwrap_or_default(),
                name_en: value.get("enName").cloned().unwrap_or_default(),
                name_local: value.get("localName").cloned().unwrap_or_default(),
            })
            .collect();

        let patterns = all
            .iter()
            .map(|area| {
                let pattern = format!("[^a-zA-Z]{}[^a-zA-Z]", area.code);
                Regex::new(&pattern).expect("区域码正则构建失败")
            })
            .collect();

        Data { all, patterns }
    })
}

/// 按节点名称匹配区域: 先匹配区域码, 再匹配中英文及本地名称。
pub fn find_match(source: &str) -> Option<&'static Area> {
    if source.is_empty() {
        return None;
    }

    let data = data();
    let upper = source.to_uppercase();

    data.all
        .iter()
        .zip(data.patterns.iter())
        .find_map(|(area, pattern)| {
            if pattern.is_match(&upper) {
                return Some(area);
            }
            if !area.name_cn.is_empty() && source.contains(&area.name_cn) {
                return Some(area);
            }
            if !area.name_en.is_empty() && source.contains(&area.name_en) {
                return Some(area);
            }
            if !area.name_local.is_empty() && source.contains(&area.name_local) {
                return Some(area);
            }
            None
        })
}
