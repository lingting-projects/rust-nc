use regex::Regex;
use serde::de::Visitor;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, OnceLock};
#[cfg(feature = "wrangler")]
use worker::{console_debug, console_error};

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

impl Serialize for Area {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.code)
    }
}

struct AreaVisitor;

impl<'de> Visitor<'de> for AreaVisitor {
    type Value = Area;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string representing an Area code")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match find(v) {
            Some(area) => Ok(area.clone()),
            None => Err(E::custom(format!("unknown area code: {}", v))),
        }
    }
}

impl<'de> Deserialize<'de> for Area {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(AreaVisitor)
    }
}

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
                        let name_cn = data.get("name").cloned().unwrap_or_default();
                        let name_en = data.get("enName").cloned().unwrap_or_default();
                        let name_local = data.get("localName").cloned().unwrap_or_default();

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

pub fn find(code: &str) -> Option<&'static Area> {
    init();
    if code.is_empty() {
        return None;
    }
    let map = MAP_CODE.get()?;
    map.get(code).map(|v| &**v)
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
                let m_r = regex.is_match(&upper);
                if m_r {
                    #[cfg(feature = "binary")]
                    log::trace!("[{}] 正则匹配成功! code: {}", source, area.code);
                    #[cfg(feature = "wrangler")]
                    console_debug!("[{}] 正则匹配成功! code: {}", source, area.code);
                    return true;
                }
                let m_cn = source.contains(&area.name_cn);
                if m_cn {
                    #[cfg(feature = "binary")]
                    log::trace!(
                        "[{}] 名称[cn]匹配成功! code: {}; name: {}",
                        source,
                        area.code,
                        area.name_cn
                    );
                    #[cfg(feature = "wrangler")]
                    console_debug!(
                        "[{}] 名称[cn]匹配成功! code: {}; name: {}",
                        source,
                        area.code,
                        area.name_cn
                    );
                    return true;
                }
                let m_en = source.contains(&area.name_en);
                if m_en {
                    #[cfg(feature = "binary")]
                    log::debug!(
                        "[{}] 名称[en]匹配成功! code: {}; name: {}",
                        source,
                        area.code,
                        area.name_en
                    );
                    #[cfg(feature = "wrangler")]
                    console_debug!(
                        "[{}] 名称[en]匹配成功! code: {}; name: {}",
                        source,
                        area.code,
                        area.name_en
                    );
                    return true;
                }
                let m_local = source.contains(&area.name_local);
                if m_local {
                    #[cfg(feature = "binary")]
                    log::debug!(
                        "[{}] 名称[local]匹配成功! code: {}; name: {}",
                        source,
                        area.code,
                        area.name_local
                    );
                    #[cfg(feature = "wrangler")]
                    console_debug!(
                        "[{}] 名称[local]匹配成功! code: {}; name: {}",
                        source,
                        area.code,
                        area.name_local
                    );
                    return true;
                }

                false
            }
            Err(e) => {
                #[cfg(feature = "binary")]
                log::error!("正则构建异常! code: {}; {}", area.code, e);
                #[cfg(feature = "wrangler")]
                console_error!("正则构建异常! code: {}; {}", area.code, e);
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
