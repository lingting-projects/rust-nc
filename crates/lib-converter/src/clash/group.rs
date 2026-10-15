use serde::Serialize;

use crate::constants::{
    CLASH_TAG_DIRECT, CLASH_TAG_REJECT, TAG_AUTO, TAG_FALLBACK, TAG_SELECTOR, TEST_URL,
};
use crate::platform::Kernel;

#[derive(Serialize)]
pub(crate) struct ProxyGroup {
    pub name: String,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    pub url: String,
    pub interval: u64,
    pub tolerance: u64,
    pub proxies: Vec<String>,
}

/// 生成代理组: 节点选择、自动选择、默认选择与各区域自动组。
pub(crate) fn build_proxy_groups(kernel: &Kernel) -> Vec<ProxyGroup> {
    let areas = build_auto_areas(kernel);
    let area_tags: Vec<String> = areas.iter().map(|group| group.name.clone()).collect();

    let auto = auto_group(TAG_AUTO.to_string(), area_tags.clone());
    let default = area_tags
        .first()
        .cloned()
        .unwrap_or_else(|| CLASH_TAG_DIRECT.to_string());

    let selector = selector_group(kernel, TAG_SELECTOR, default, &area_tags);
    let fallback = selector_group(
        kernel,
        TAG_FALLBACK,
        CLASH_TAG_DIRECT.to_string(),
        &area_tags,
    );

    let mut groups = vec![selector, auto, fallback];
    groups.extend(areas);
    groups
}

fn build_auto_areas(kernel: &Kernel) -> Vec<ProxyGroup> {
    kernel
        .map_area()
        .into_iter()
        .map(|(code, nodes)| {
            let area = nodes[0].area.expect("区域分组中的节点必然有区域");
            let name = format!("[{}] {}自动", code, area.name_cn);
            let proxies = nodes.iter().map(|node| node.name()).collect();
            auto_group(name, proxies)
        })
        .collect()
}

fn auto_group(name: String, proxies: Vec<String>) -> ProxyGroup {
    ProxyGroup {
        name,
        type_: "url-test".to_string(),
        default: None,
        url: TEST_URL.to_string(),
        interval: 1800,
        tolerance: 150,
        proxies,
    }
}

fn selector_group(
    kernel: &Kernel,
    name: &str,
    default: String,
    area_tags: &[String],
) -> ProxyGroup {
    let mut proxies = vec![
        CLASH_TAG_DIRECT.to_string(),
        CLASH_TAG_REJECT.to_string(),
        TAG_AUTO.to_string(),
    ];
    proxies.extend(area_tags.iter().cloned());
    proxies.extend(kernel.nodes.iter().map(|node| node.name()));

    ProxyGroup {
        name: name.to_string(),
        type_: "select".to_string(),
        default: Some(default),
        url: TEST_URL.to_string(),
        interval: 1800,
        tolerance: 150,
        proxies,
    }
}
