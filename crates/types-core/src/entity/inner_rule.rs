use framework_proc_auto::auto_type;

/// 内核内置规则, 对应 tbl_inner_rule。
///
/// 内核配置引用的远程规则集需要提前落到本地, 否则内核启动时会卡在规则集下载,
/// API 服务迟迟不监听。`ready` 为 `false` 表示本地文件尚未可用。
#[auto_type(default = false)]
pub struct InnerRule {
    pub id: String,
    /// 原始下载地址, 不含加速镜像前缀
    pub url: String,
    /// 最近一次下载成功时远端返回的 etag
    pub etag: String,
    /// 本地文件是否可用
    pub ready: bool,
}
