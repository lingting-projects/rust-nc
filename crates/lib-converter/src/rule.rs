//! 规则类型与来源。

/// 规则类型, 与规则文件一一对应。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleType {
    /// IP 规则
    Ip,
    /// 进程规则
    Process,
    /// 其他规则
    Other,
}

/// 规则来源。
#[derive(Debug, Clone)]
pub struct Rule {
    /// 本地文件路径或远程地址
    pub path: String,
    pub rule_type: RuleType,
    /// 是否来自远程
    pub remote: bool,
}

impl Rule {
    /// 本地规则。
    pub fn from_local(rule_type: RuleType, path: String) -> Self {
        Self {
            rule_type,
            path,
            remote: false,
        }
    }

    /// 远程规则, 地址会经过 github 加速处理。
    pub fn from_remote(rule_type: RuleType, url: String) -> Self {
        Self {
            rule_type,
            path: crate::constants::fast(&url),
            remote: true,
        }
    }
}
