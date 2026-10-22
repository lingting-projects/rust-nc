use anyhow::Result;
use lib_biz::config::ConfigBiz;
use lib_biz::config_sync::ConfigSync;
use lib_biz::timer;
use lib_core::LogLevel;
use service_core::manager::setting_manager::SettingManager;
use service_core::service::config_service::ConfigService;
use service_core::service::rule_service::RuleService;
use service_core::service::subscribe_service::SubscribeService;
use types_core::dto::config::ConfigUpsertDTO;
use types_core::dto::setting::SettingKernel;
use types_core::entity::config::Config;
use types_core::entity::rule::Rule;
use types_core::entity::subscribe::Subscribe;

use super::Services;

/// 配置页面一次加载所需的全部数据。
///
/// 配置、订阅与规则一起读取，订阅与规则供新增、编辑弹窗的下拉框使用。
pub struct ConfigPageData {
    pub configs: Vec<Config>,
    pub subscribes: Vec<Subscribe>,
    pub rules: Vec<Rule>,
}

impl Services {
    /// 读取配置列表，以及表单需要的订阅与规则。
    pub async fn config_page_data(self: &std::sync::Arc<Self>) -> Result<ConfigPageData> {
        let context = self.context();
        let configs = ConfigService::new(context.clone()).list().await?;
        let subscribes = SubscribeService::new(context.clone()).list().await?;
        let rules = RuleService::new(context).list().await?;

        Ok(ConfigPageData {
            configs,
            subscribes,
            rules,
        })
    }
}

impl Services {
    /// 同步配置，未指定 id 时同步全部。
    ///
    /// 同步全部只负责推送，等待由后台任务继续完成。
    pub async fn sync_config(&self, id: Option<&str>) -> Result<usize> {
        ConfigBiz::new(self.context()).sync(id).await
    }
}

impl Services {
    /// 新增配置，写库后触发一次同步，返回配置 id。
    pub async fn create_config(&self, dto: ConfigUpsertDTO) -> Result<String> {
        ConfigBiz::new(self.context()).create(dto).await
    }

    /// 修改配置，写库后触发一次同步，返回配置 id。
    pub async fn edit_config(&self, dto: ConfigUpsertDTO) -> Result<String> {
        ConfigBiz::new(self.context()).edit(dto).await
    }

    /// 删除一条配置。
    pub async fn delete_config(&self, id: &str) -> Result<()> {
        ConfigBiz::new(self.context()).delete(id).await?;
        Ok(())
    }
}

impl Services {
    /// 把数据库上下文与运行时注入配置同步调度器。
    pub fn init_config_sync(&self) {
        ConfigSync::instance().init(self.context(), self.runtime_handle());
    }
}

impl Services {
    /// 注入定时同步环境并启动定时任务运行时。
    ///
    /// 定时任务按固定间隔扫描到达刷新时间的数据并推入同步队列;
    /// 启动失败只记录日志, 手动同步不受影响。
    pub fn init_timer(&self) {
        timer::init(self.context(), self.runtime_handle());
        if let Err(error) = timer::start() {
            eprintln!("启动定时同步失败: {error:#}");
        }
    }
}

impl Services {
    /// 读取内核设置，供设置页面的内核分区展示。
    pub async fn kernel_setting(&self) -> Result<SettingKernel> {
        SettingManager::new(self.context(), self.cache_dir.clone())
            .kernel()
            .await
    }

    /// 只写入内核日志等级。
    ///
    /// 等级只影响后续生成的配置，用户需要自行刷新配置并重启内核后才生效。
    pub async fn save_kernel_log_level(&self, log_level: LogLevel) -> Result<()> {
        SettingManager::new(self.context(), self.cache_dir.clone())
            .upsert_kernel_log_level(log_level)
            .await
    }

    /// 只写入内核监听设置。
    pub async fn save_kernel_listen(
        &self,
        ui_listen: &str,
        ui_port: u16,
        mixed_listen: &str,
        mixed_port: u16,
    ) -> Result<()> {
        SettingManager::new(self.context(), self.cache_dir.clone())
            .upsert_kernel_listen(ui_listen, ui_port, mixed_listen, mixed_port)
            .await
    }
}
