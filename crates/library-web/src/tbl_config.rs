pub struct TblConfig {
    pub id: String,
    pub name: String,
    pub tun: bool,
    pub fake_ip: bool,
    pub ipv6: bool,
    /// 订阅
    pub subscribe_id: String,
    /// geo cn 直连
    pub geo_cn: bool,
    /// 规则 id1,id2
    pub rule_direct_ids: String,
    pub rule_proxy_ids: String,
    pub rule_reject_ids: String,
    /// 包含
    pub include_area_non: bool,
    /// 包含指定区域 json字符串
    pub include_area: Vec<String>,
    /// 包含名称中存在关键的 json字符串
    pub include_name_contains: Vec<String>,
    /// 排除
    /// 排除指定区域 json字符串
    pub exclude_area: Vec<String>,
    /// 排除名称中存在关键的 json字符串
    pub exclude_name_contains: Vec<String>,
    /// 刷新间隔, 单位: 毫秒
    pub interval: u32,
    /// 刷新时间
    pub refresh_time: u128,
    /// 更新时间: 毫秒级别时间戳
    pub update_time: u128,
    /// 创建时间: 毫秒级别时间戳
    pub create_time: u128,
}
