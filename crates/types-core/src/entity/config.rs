use framework_proc_auto::auto_type;

/// 内核使用的配置, 对应 tbl_config
#[auto_type(default = false)]
pub struct Config {
    pub id: String,
    pub name: String,
    /// 是否启用 tun
    pub tun: bool,
    /// 是否启用 fake ip
    pub fake_ip: bool,
    /// 是否启用 ipv6
    pub ipv6: bool,
    /// 订阅 id
    pub subscribe_id: String,
    /// geo cn 直连
    pub geo_cn: bool,
    /// 直连规则 id 集合
    pub rule_direct_ids: Vec<String>,
    /// 代理规则 id 集合
    pub rule_proxy_ids: Vec<String>,
    /// 拒绝规则 id 集合
    pub rule_reject_ids: Vec<String>,
    /// 包含: 是否包含无区域节点
    pub include_area_non: bool,
    /// 包含指定区域
    pub include_area: Vec<String>,
    /// 包含名称中存在关键字的节点
    pub include_name_contains: Vec<String>,
    /// 排除指定区域
    pub exclude_area: Vec<String>,
    /// 排除名称中存在关键字的节点
    pub exclude_name_contains: Vec<String>,
    /// 刷新间隔, 单位: 毫秒
    pub interval: i64,
    /// 刷新时间: 毫秒级别时间戳
    pub refresh_time: i64,
    /// 更新时间: 毫秒级别时间戳
    pub update_time: i64,
    /// 创建时间: 毫秒级别时间戳
    pub create_time: i64,
}
