use framework_proc_auto::{auto_enum, auto_type};

/// 规则类型, 与 sing-box 规则文件一一对应
#[auto_enum]
pub enum RuleType {
    /// IP 规则
    #[strum(serialize = "ip")]
    #[serde(rename = "ip")]
    Ip,
    /// 进程规则
    #[strum(serialize = "process")]
    #[serde(rename = "process")]
    Process,
    /// 其他规则
    #[strum(serialize = "other")]
    #[serde(rename = "other")]
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

/// 规则, 对应 tbl_rule。
///
/// `url` 为空表示本地规则, 此时 `content` 为本地编辑内容。
/// `count` 为可用规则总数, 另按 [`RuleType`] 分别记录数量。
#[auto_type(default = false)]
pub struct Rule {
    pub id: String,
    pub name: String,
    /// 规则订阅地址, 为空表示本地规则
    pub url: String,
    /// 订阅完整内容(url返回或者本地编辑的)
    pub content: String,
    /// 刷新间隔, 单位: 毫秒
    pub interval: i64,
    /// 更新时间: 毫秒级别时间戳
    pub update_time: i64,
    /// 创建时间: 毫秒级别时间戳
    pub create_time: i64,
    /// 刷新时间: 毫秒级别时间戳
    pub refresh_time: i64,
    /// 可用规则数量
    pub count: u64,
    /// 进程规则数量
    pub count_process: u64,
    /// IP 规则数量
    pub count_ip: u64,
    /// 其他规则数量
    pub count_other: u64,
}
