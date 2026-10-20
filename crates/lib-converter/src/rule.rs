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

impl RuleType {
    /// 规则文件使用的名称
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Ip => "ip",
            Self::Process => "process",
            Self::Other => "other",
        }
    }
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
    ///
    /// 仅在规则由内核自行下载时使用; 内核配置中的远程规则集统一改由内置规则
    /// 提前落盘为本地文件, 避免内核启动被下载拖住。
    pub fn from_remote(rule_type: RuleType, url: String) -> Self {
        Self {
            rule_type,
            path: crate::constants::fast(&url),
            remote: true,
        }
    }
}
