//! 连接列表的展示辅助。
//!
//! 连接数据由 `lib-singbox` 的常驻订阅维护，页面只持有快照的共享引用，
//! 因此这里只保留「取一行展示文本」与「按关键字筛选」这类纯展示逻辑。

use lib_singbox::ConnectionRecord;

/// 链路文本，从第一个出站到最后一个出站。
pub fn chain(row: &ConnectionRecord) -> String {
    row.chain()
}

/// 进程名，只取路径的最后一段；取不到进程信息时返回 `-`。
pub fn process(row: &ConnectionRecord) -> String {
    if row.info.process.is_empty() {
        return "-".to_string();
    }
    let name = row
        .info
        .process
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or_default();
    match name.is_empty() {
        true => row.info.process.clone(),
        false => name.to_string(),
    }
}

/// 是否命中关键字：按入口、进程、目标、规则、链路与源地址做包含匹配。
pub fn matched(row: &ConnectionRecord, keyword: &str) -> bool {
    let keyword = keyword.trim().to_lowercase();
    if keyword.is_empty() {
        return true;
    }

    let fields = [
        row.info.inbound.as_str(),
        row.info.destination.as_str(),
        row.info.rule.as_str(),
        row.info.source.as_str(),
        row.info.process.as_str(),
    ];
    if fields
        .iter()
        .any(|field| field.to_lowercase().contains(&keyword))
    {
        return true;
    }

    row.info
        .chain
        .iter()
        .any(|item| item.to_lowercase().contains(&keyword))
}

/// 关键字命中的行下标。
pub fn filter(rows: &[ConnectionRecord], keyword: &str) -> Vec<usize> {
    (0..rows.len())
        .filter(|ix| matched(&rows[*ix], keyword))
        .collect()
}
