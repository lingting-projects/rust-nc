use lib_core::current_millis;
use lib_core::data_size::DataSize;

/// 数据大小文本，0 表示没有数据，显示为 `-`。
pub fn data_size(bytes: u64) -> String {
    if bytes == 0 {
        return "-".to_string();
    }
    DataSize::of_bytes(bytes).display()
}

/// 每秒速率文本，用于实时监控的瞬时值。
pub fn speed(bytes_per_second: f64) -> String {
    if bytes_per_second <= 0. {
        return "0 B/s".to_string();
    }
    format!(
        "{}/s",
        DataSize::of_bytes(bytes_per_second as u64).display()
    )
}

/// 剩余量文本：总量减去已用量，已用超过总量时带负号。
pub fn remaining(max: u64, used: u64) -> String {
    if max == 0 {
        return "-".to_string();
    }
    if max < used {
        return format!("-{}", DataSize::of_bytes(used - max).display());
    }
    DataSize::of_bytes(max - used).display()
}

/// 距离 `millis` 还有多久，已经过去时返回 `已过期`。
pub fn until(millis: i64) -> String {
    match duration(millis - now()) {
        Some(text) => text,
        None => "已过期".to_string(),
    }
}

/// 距离 `millis` 过去了多久，追加 `前`。
pub fn since(millis: i64) -> String {
    if millis <= 0 {
        return "-".to_string();
    }
    match duration(now() - millis) {
        Some(text) => format!("{text}前"),
        None => "-".to_string(),
    }
}

/// 当前毫秒时间戳，取不到时退化为 0。
fn now() -> i64 {
    current_millis().unwrap_or_default()
}

/// 把毫秒差值表达成「数值 + 单位」，负数表示时间点已经过去。
///
/// 单位阶梯与旧界面一致：秒、分钟、小时、天、月（按 31 天）、年（按 12 月）。
fn duration(millis: i64) -> Option<String> {
    if millis < 0 {
        return None;
    }
    if millis < 1_000 {
        return Some("刚刚".to_string());
    }

    let seconds = millis / 1_000;
    if seconds < 60 {
        return Some(format!("{seconds} 秒钟"));
    }
    let minutes = seconds / 60;
    if minutes < 60 {
        return Some(format!("{minutes} 分钟"));
    }
    let hours = minutes / 60;
    if hours < 24 {
        return Some(format!("{hours} 小时"));
    }
    let days = hours / 24;
    if days < 7 {
        return Some(format!("{days} 天"));
    }
    let months = days / 31;
    if months < 12 {
        return Some(format!("{months} 月"));
    }

    Some(format!("{} 年", months / 12))
}
