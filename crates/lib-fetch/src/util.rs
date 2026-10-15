use std::collections::HashMap;

use lib_core::MultiStringValue;

/// 请求头展开为 (名称, 值) 列表, 同名请求头会有多条。
pub(crate) fn pairs(headers: &MultiStringValue) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    headers.for_each(|name, values| {
        for value in values {
            pairs.push((name.clone(), value.clone()));
        }
    });
    pairs
}

/// 由 (名称, 值) 列表构建 [`MultiStringValue`], 名称统一小写。
pub(crate) fn to_multi_string_value<I, K, V>(items: I) -> MultiStringValue
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<str>,
    V: AsRef<str>,
{
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for (name, value) in items {
        map.entry(name.as_ref().to_ascii_lowercase())
            .or_default()
            .push(value.as_ref().to_string());
    }

    MultiStringValue::create(true, map)
}
