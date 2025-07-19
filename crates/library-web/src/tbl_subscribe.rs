pub struct TblSubscribe {
    id: String,
    name: String,
    /// 订阅地址
    url: String,
    /// 订阅地址返回完整内容
    content: String,
    /// 订阅解析后的所有节点
    nodes: String,
    /// 更新间隔, 单位: 毫秒
    interval: u32,
    /// 更新时间: 毫秒级别时间戳
    update_time: u64,
    /// 创建时间: 毫秒级别时间戳
    create_time: u64,
    /// 下载流量, 单位: Bytes
    download: u64,
    /// 上传流量, 单位: Bytes
    upload: u64,
    /// 最大可用流量, 单位: Bytes
    max: u64,
    /// 过期时间: 毫秒级别时间戳
    expire_time: u64,
}
