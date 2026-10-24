use framework_proc_auto::auto_type;
use lib_core::constants;

/// 新增或修改内核配置的入参。
///
/// `id` 为空表示新增。
#[auto_type(default = false)]
pub struct ConfigUpsertDTO {
    pub id: Option<i64>,
    pub name: String,
    /// 是否启用 tun
    pub tun: bool,
    /// 是否启用 fake ip
    pub fake_ip: bool,
    /// 是否启用 ipv6
    pub ipv6: bool,
    /// 订阅 id, 未选择订阅时为空
    pub subscribe_id: Option<i64>,
    /// geo cn 直连
    pub geo_cn: bool,
    /// 直连规则 id 集合
    pub rule_direct_ids: Vec<i64>,
    /// 代理规则 id 集合
    pub rule_proxy_ids: Vec<i64>,
    /// 拒绝规则 id 集合
    pub rule_reject_ids: Vec<i64>,
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
}

impl Default for ConfigUpsertDTO {
    fn default() -> Self {
        Self {
            id: None,
            name: String::new(),
            tun: true,
            fake_ip: true,
            ipv6: true,
            subscribe_id: None,
            geo_cn: true,
            rule_direct_ids: Vec::new(),
            rule_proxy_ids: Vec::new(),
            rule_reject_ids: Vec::new(),
            include_area_non: constants::NODE_INCLUDE_AREA_NON,
            include_area: to_vec(constants::NODE_INCLUDE_AREA),
            include_name_contains: Vec::new(),
            exclude_area: to_vec(constants::NODE_EXCLUDE_AREA),
            exclude_name_contains: to_vec(constants::NODE_EXCLUDE_NAME_CONTAINS),
        }
    }
}

fn to_vec(source: &[&str]) -> Vec<String> {
    source.iter().map(|item| item.to_string()).collect()
}
