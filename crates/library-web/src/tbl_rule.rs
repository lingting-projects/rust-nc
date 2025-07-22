pub struct TblRule {
    id: String,
    name: String,
    /// 规则订阅地址, 空表示本地规则
    url: String,
    /// 订阅完整内容(url返回或者本地编辑的)
    content: String,
    /// 更新间隔, 单位: 毫秒
    interval: u32,
    /// 更新时间: 毫秒级别时间戳
    update_time: u128,
    /// 创建时间: 毫秒级别时间戳
    create_time: u128,
    /// 可用规则数量
    count: u64,
    /// 进程规则数量
    count_process: u64,
    /// IP规则数量
    count_ip: u64,
    /// 其他规则数量
    count_other: u64,
}
