use std::sync::OnceLock;

use framework_region::regions::REGION_REGIONS;
use framework_region::types::Region;
use regex::Regex;

static PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();

fn patterns() -> &'static [Regex] {
    PATTERNS.get_or_init(|| {
        REGION_REGIONS
            .iter()
            .map(|region| {
                let pattern = format!("[^a-zA-Z]{}[^a-zA-Z]", region.iso);
                Regex::new(&pattern).expect("区域码正则构建失败")
            })
            .collect()
    })
}

/// 按节点名称匹配区域: 先匹配区域码, 再匹配中英文及本地名称。
pub fn find_match(source: &str) -> Option<&'static Region> {
    if source.is_empty() {
        return None;
    }

    let upper = source.to_uppercase();

    REGION_REGIONS
        .iter()
        .zip(patterns())
        .find_map(|(region, pattern)| {
            if pattern.is_match(&upper) {
                return Some(region);
            }
            if !region.name.zh.is_empty() && source.contains(region.name.zh) {
                return Some(region);
            }
            if !region.name.en.is_empty() && source.contains(region.name.en) {
                return Some(region);
            }
            None
        })
}
