use std::collections::HashMap;

use lib_converter::NodeContains;
use lib_pick::is_true;

/// 转换请求参数。
pub struct ConvertParams {
    /// 订阅远程地址, `s:` 前缀表示分享源
    pub remote: String,
    pub tun: bool,
    pub ipv6: bool,
    pub fake_ip: bool,
    pub geo_cn: bool,
    pub debug: bool,
    /// 节点包含条件
    pub include: NodeContains,
    /// 节点排除条件
    pub exclude: NodeContains,
}

impl ConvertParams {
    /// 由查询参数解析, `resolve_remote` 用于把分享源解析为真实地址。
    pub fn from_query(
        source: HashMap<String, Vec<String>>,
        resolve_remote: impl FnOnce(&str, &str) -> Option<String>,
    ) -> Option<Self> {
        let remote = Self::first(&source, "remote")?;
        let remote = Self::remote(&remote, resolve_remote)?;

        let mut exclude = Self::contains("exclude", &source, &NodeContains::default_exclude());
        if exclude.is_empty() {
            exclude = NodeContains::default_exclude();
        }

        let include = if Self::convert_bool(source.get("include.only_main")).unwrap_or(false) {
            NodeContains::default_include()
        } else {
            Self::contains("include", &source, &NodeContains::default_include())
        };

        Some(Self {
            remote,
            tun: Self::convert_bool(source.get("tun")).unwrap_or(true),
            ipv6: Self::convert_bool(source.get("ipv6")).unwrap_or(true),
            fake_ip: Self::convert_bool(source.get("fake_ip")).unwrap_or(true),
            geo_cn: Self::convert_bool(source.get("geo_cn")).unwrap_or(true),
            debug: Self::convert_bool(source.get("debug")).unwrap_or(false),
            include,
            exclude,
        })
    }

    /// 解析远程地址, `s:key?p` 表示分享源。
    fn remote(source: &str, resolve: impl FnOnce(&str, &str) -> Option<String>) -> Option<String> {
        let Some(source) = source.strip_prefix("s:") else {
            return Some(source.to_string());
        };

        let (key, p) = match source.split_once('?') {
            Some((key, p)) => (key, p),
            None => (source, ""),
        };
        resolve(key, p)
    }

    /// 解析布尔参数。
    fn convert_bool(option: Option<&Vec<String>>) -> Option<bool> {
        Some(is_true(option?.first()?))
    }

    /// 读取参数的全部非空值。
    fn get_all(source: &HashMap<String, Vec<String>>, key: &str) -> Vec<String> {
        let mut values = Vec::new();
        if let Some(items) = source.get(key) {
            for item in items {
                let item = item.trim();
                if !item.is_empty() {
                    values.push(item.to_string());
                }
            }
        }
        values
    }

    /// 读取第一个参数值。
    fn first(source: &HashMap<String, Vec<String>>, key: &str) -> Option<String> {
        source.get(key)?.first().cloned()
    }

    /// 由 `prefix.area` 与 `prefix.name_contains` 构建筛选条件, 其余沿用默认值。
    fn contains(
        prefix: &str,
        source: &HashMap<String, Vec<String>>,
        default: &NodeContains,
    ) -> NodeContains {
        NodeContains {
            area: Self::get_all(source, &format!("{prefix}.area")),
            name_contains: Self::get_all(source, &format!("{prefix}.name_contains")),
            non_area: default.non_area,
            non_name: default.non_name,
        }
    }
}
