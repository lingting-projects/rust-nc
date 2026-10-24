use std::sync::Arc;

use anyhow::{Context as _, Result};
use lib_biz::inner_rule_sync::InnerRuleSync;
use lib_biz::kernel::{SingBoxInstance, Status};
use lib_singbox::{ConnectionSnapshot, LogBlock, LogSnapshot};

use super::Services;

/// 内核状态快照。
pub struct KernelData {
    pub status: Status,
    /// 正在运行的配置 id，未运行时为 `None`。
    pub config_id: Option<String>,
    /// 最近一次异常的原因。
    pub reason: Option<String>,
    /// 是否正在执行启停操作，任意入口发起都算。
    pub acting: bool,
}

/// 内核运行状态快照，取自内核 gRPC API。
pub struct KernelStatus {
    /// 内核进程内存占用，单位 Bytes。
    pub memory: u64,
    /// 内核启动以来的累计上传，单位 Bytes。
    pub upload: u64,
    /// 内核启动以来的累计下载，单位 Bytes。
    pub download: u64,
}

/// 读取内核状态。
///
/// 内核实例全局唯一，数据层未就绪时也能读到状态，只是没有可启动的配置。
pub fn kernel_data() -> KernelData {
    let kernel = SingBoxInstance::instance();
    KernelData {
        status: kernel.status(),
        config_id: kernel.config_id(),
        reason: kernel.reason(),
        acting: kernel.acting(),
    }
}

impl Services {
    /// 把数据库上下文注入全局内核实例与内置规则同步调度器。
    ///
    /// 内置规则必须先于内核启动下载完成, 因此与内核实例一起初始化。
    pub fn init_kernel(&self) {
        InnerRuleSync::instance().init(self.context(), self.runtime_handle());
        SingBoxInstance::instance().init(self.context());
    }
}
impl Services {
    /// 启动配置 `id` 的内核，内核 API 探测通过后才返回成功。
    pub async fn start_kernel(&self, id: &str) -> Result<()> {
        SingBoxInstance::instance().start(id).await
    }

    /// 停止内核。
    pub async fn stop_kernel(&self) -> Result<()> {
        SingBoxInstance::instance().stop().await
    }
}

impl Services {
    /// 读取一次内核运行状态，内核未运行时返回 `None`。
    ///
    /// 状态来自内核 gRPC API，包含内存占用与累计流量；
    /// 内核未运行或 API 不可用时没有可读的数据。
    pub async fn kernel_status(&self) -> Result<Option<KernelStatus>> {
        let singbox = SingBoxInstance::instance().singbox()?;
        let Some(grpc) = singbox.grpc()? else {
            return Ok(None);
        };

        let status = grpc.status().await.context("读取内核运行状态失败")?;
        Ok(Some(KernelStatus {
            memory: status.memory,
            upload: status.uplink_total.max(0) as u64,
            download: status.downlink_total.max(0) as u64,
        }))
    }
}

/// 出站分组的一个节点。
#[derive(Clone)]
pub struct GroupNode {
    pub tag: String,
    pub kind: String,
    /// 最近一次测速的延迟，单位毫秒，0 表示尚未测速。
    pub delay: i32,
}

/// 出站分组。
#[derive(Clone)]
pub struct GroupInfo {
    /// 分组标签，切换节点时的分组标识。
    pub tag: String,
    /// 是否支持手动切换节点。
    pub selectable: bool,
    /// 当前选中的节点标签。
    pub selected: String,
    /// 分组内的节点。
    pub items: Vec<GroupNode>,
}

impl GroupInfo {
    /// 当前选中节点的延迟，节点不在列表里时为 0。
    pub fn delay(&self) -> i32 {
        self.items
            .iter()
            .find(|item| item.tag == self.selected)
            .map(|item| item.delay)
            .unwrap_or_default()
    }
}

impl Services {
    /// 读取出站分组。
    pub async fn kernel_groups(&self) -> Result<Vec<GroupInfo>> {
        let groups = SingBoxInstance::instance().groups().await?;
        Ok(groups
            .into_iter()
            .map(|group| GroupInfo {
                tag: group.tag,
                selectable: group.selectable,
                selected: group.selected,
                items: group
                    .items
                    .into_iter()
                    .map(|item| GroupNode {
                        tag: item.tag,
                        kind: item.r#type,
                        delay: item.url_test_delay,
                    })
                    .collect(),
            })
            .collect())
    }

    /// 在分组中切换到指定节点。
    pub async fn kernel_select(&self, group_tag: &str, outbound_tag: &str) -> Result<()> {
        SingBoxInstance::instance()
            .select(group_tag, outbound_tag)
            .await
    }

    /// 推送一个节点到测速队列，返回是否成功入队（重复推送会被去重）。
    pub fn kernel_url_test(&self, outbound_tag: &str) -> Result<bool> {
        SingBoxInstance::instance().url_test(outbound_tag)
    }
}

impl Services {
    /// 当前连接快照。
    ///
    /// 内核的 `SubscribeConnections` 由 `lib-singbox` 常驻订阅，推送结果累积成一份
    /// 不可变快照；这里只把共享引用交给界面，不重新建流，也不复制连接数据。
    pub fn kernel_connections(&self) -> Result<Arc<ConnectionSnapshot>> {
        let store = SingBoxInstance::instance().connections()?;
        Ok(store.snapshot().unwrap_or_default())
    }

    /// 清空内核侧缓存的全部连接，之后只保留新推送的数据。
    ///
    /// 只清 `lib-singbox` 缓存的连接，不关闭内核里的连接。
    pub fn kernel_clear_connections(&self) -> Result<()> {
        SingBoxInstance::instance().clear_connections()
    }

    /// 关闭指定连接。
    pub async fn kernel_close_connection(&self, id: &str) -> Result<()> {
        SingBoxInstance::instance().close_connection(id).await
    }
}

impl Services {
    /// 当前内核日志快照。
    ///
    /// 日志文件由 `lib-singbox` 的常驻线程增量读取并解析成块，
    /// 这里只把共享引用交给界面，不重新读文件，也不复制日志内容。
    pub fn kernel_logs(&self) -> Result<Arc<LogSnapshot>> {
        let store = SingBoxInstance::instance().logs()?;
        Ok(store.snapshot().unwrap_or_default())
    }
}

impl Services {
    /// 读取一个日志块的完整内容，日志文件已被重写时返回 `None`。
    ///
    /// `progress` 每读完一块调用一次并汇报已读字节，返回 `false` 时立即停止读取。
    pub fn kernel_log_detail(
        &self,
        block: &LogBlock,
        progress: impl FnMut(u64) -> bool,
    ) -> Result<Option<String>> {
        SingBoxInstance::instance().log_detail(block, progress)
    }
}
