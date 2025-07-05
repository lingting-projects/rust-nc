use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

// 修正类型定义：使用HashMap而非Map
static ALL: OnceLock<Vec<Area>> = OnceLock::new();
static MAP_CODE: OnceLock<HashMap<String, &Area>> = OnceLock::new();

fn init() {
    match ALL.get() {
        Some(_) => {}
        None => {
            let areas = ALL.get_or_init(|| {
                let json = include_str!("../../../assets/countries.json");
                let root: HashMap<String, HashMap<String, String>> =
                    serde_json::from_str(json).expect("Failed to parse countries.json");

                root.into_iter()
                    .filter_map(|(code, data)| {
                        // 提取各语言名称，使用默认值防止缺失
                        let name_cn = data.get("name_cn").cloned().unwrap_or_default();
                        let name_en = data.get("name_en").cloned().unwrap_or_default();
                        let name_local = data.get("name_local").cloned().unwrap_or_default();

                        Some(Area {
                            code,
                            name_cn,
                            name_en,
                            name_local,
                        })
                    })
                    .collect()
            });

            let map_code: HashMap<String, &Area> =
                areas.iter().map(|area| (area.code.clone(), area)).collect();

            MAP_CODE.set(map_code).expect("Failed set code map");
        }
    }
}

pub fn find(code: Option<String>) -> Option<&'static Area> {
    let code = code?;
    init();
    if code.is_empty() {
        return None;
    }
    let map = MAP_CODE.get()?;
    map.get(&code).map(|v| &**v)
}

pub fn find_match(source: &str) -> Option<&'static Area> {
    init();
    if source.is_empty() {
        return None;
    }
    let all = ALL.get()?;
    let upper = source.to_uppercase();
    all.iter().find(|area| {
        let pattern = format!("[^a-zA-Z]{}[^a-zA-Z]", area.code);
        match Regex::new(&pattern) {
            Ok(regex) => {
                regex.is_match(&upper)
                    || source.contains(&area.name_cn)
                    || source.contains(&area.name_en)
                    || source.contains(&area.name_local)
            }
            Err(e) => {
                #[cfg(feature = "log")]
                log::error!("正则构建异常! code: {}; {}", area.code, e);
                false
            }
        }
    })
}

pub fn find_name(name: Option<String>) -> Option<&'static Area> {
    let name = name?;
    init();
    if name.is_empty() {
        return None;
    }
    let all = ALL.get()?;
    all.iter().find(|area| {
        area.name_cn.contains(&name)
            || area.name_en.contains(&name)
            || area.name_local.contains(&name)
    })
}
