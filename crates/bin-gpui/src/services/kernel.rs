use anyhow::Result;
use lib_biz::inner_rule_sync::InnerRuleSync;
use lib_biz::kernel::{SingBoxInstance, Status};

use super::Services;

/// 内核状态快照。
pub struct KernelData {
    pub status: Status,
    /// 正在运行的配置 id，未运行时为 `None`。
    pub config_id: Option<String>,
    /// 最近一次异常的原因。
    pub reason: Option<String>,
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
