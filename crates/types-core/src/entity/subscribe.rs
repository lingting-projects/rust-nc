use framework_proc_auto::auto_type;

/// 订阅, 对应 tbl_subscribe
#[auto_type(default = false)]
pub struct Subscribe {
    pub id: String,
    pub name: String,
    /// 订阅地址, 为空表示本地订阅
    pub url: String,
    /// 订阅地址返回的完整内容
    pub content: String,
    /// 订阅解析后的所有节点, JSON 文本
    pub nodes: String,
    /// 刷新间隔, 单位: 毫秒
    pub interval: i64,
    /// 更新时间: 毫秒级别时间戳
    pub update_time: i64,
    /// 创建时间: 毫秒级别时间戳
    pub create_time: i64,
    /// 刷新时间: 毫秒级别时间戳
    pub refresh_time: i64,
    /// 下载流量, 单位: Bytes
    pub download: u64,
    /// 上传流量, 单位: Bytes
    pub upload: u64,
    /// 最大可用流量, 单位: Bytes
    pub max: u64,
    /// 过期时间: 毫秒级别时间戳
    pub expire_time: i64,
}
