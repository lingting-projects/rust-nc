pub struct TblConfig {
    id: String,
    name: String,
    tun: bool,
    fake_ip: bool,
    ipv6: bool,
    /// 订阅
    subscribe_id: String,
    /// geo cn 直连
    geo_cn: bool,
    /// 规则 id1,id2
    rule_direct_ids: String,
    rule_proxy_ids: String,
    rule_reject_ids: String,
    /// 包含
    include_area_non: bool,
    /// 包含指定区域 json字符串
    include_area: Vec<String>,
    /// 包含名称中存在关键的 json字符串
    include_name_contains: Vec<String>,
    /// 排除
    /// 排除指定区域 json字符串
    exclude_area: Vec<String>,
    /// 排除名称中存在关键的 json字符串
    exclude_name_contains: Vec<String>,
    /// 更新时间: 毫秒级别时间戳
    update_time: u128,
    /// 创建时间: 毫秒级别时间戳
    create_time: u128,
}
