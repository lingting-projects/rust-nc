//! 自动运行内核前的就绪等待。
//!
//! 旧数据迁移不同步刷新时间, 迁移后首次启动时配置、订阅与规则都还是刷新时间 0,
//! 此时启动内核会用到尚未生成的配置文件。这里等到选中配置的刷新时间非 0 再返回,
//! 由调用方继续自动运行: 依赖未就绪时先推动依赖自身同步, 依赖就绪后推动全部配置刷新,
//! 如此循环直到配置刷新完成。刷新时间非 0 说明该数据已经同步过一次, 因此直接放行。
//!
//! 迁移后首次运行时内置规则也尚未登记, 等待期间一并登记并推入下载。

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use lib_db::DbContext;
use log::{debug, error, info};
use service_core::service::config_service::ConfigService;
use service_core::service::rule_service::RuleService;
use service_core::service::subscribe_service::SubscribeService;
use types_core::entity::config::Config;

use crate::config::ConfigBiz;
use crate::inner_rule;
use crate::kernel::{SingBoxInstance, Status};
use crate::rule_sync::RuleSync;
use crate::subscribe_sync::SubscribeSync;

/// 就绪检查的轮询间隔。
const STEP: Duration = Duration::from_millis(300);

/// 等待配置 `id` 刷新就绪, 返回是否可以继续自动运行。
///
/// 返回 `false` 表示等待期间内核已被其他入口启动, 本次自动运行应当放弃。
/// 配置已被删除、未选择订阅或引用的订阅已不存在时返回错误, 调用方据此放弃自动运行:
/// 这类配置生成不出内核配置, 启动内核只会失败。
pub async fn wait_ready(context: &Arc<DbContext>, id: i64) -> Result<bool> {
    let mut stage = None;
    // 内置规则只登记一次, 避免轮询期间反复写库
    let mut registered = false;
    loop {
        // 等待期间用户手动启动内核时不再插手, 避免与手动操作抢占启停
        if SingBoxInstance::instance().status() != Status::Stopped {
            return Ok(false);
        }

        let config = find(context, id).await?;
        // 自启动配置的 refresh_time = 0 只有从旧版本升级的数据才有
        if config.refresh_time != 0 {
            if stage.is_some() {
                info!("[启动] 配置「{}」已刷新完成", config.name);
            }
            return Ok(true);
        }

        // 迁移后首次运行时内置规则还没登记, 只在本轮补齐一次并推入下载,
        // 否则配置生成后内核会引用不存在的本地规则集
        if !registered {
            registered = true;
            if let Err(error) = inner_rule::register_all(context).await {
                error!("[启动] 登记内置规则失败! {error:#}");
                return Err(error);
            }
        }

        let dependencies = dependencies(context, &config).await?;
        if dependencies.ready() {
            if stage != Some(Stage::Config) {
                stage = Some(Stage::Config);
                info!(
                    "[启动] 配置「{}」的依赖已就绪, 等待配置刷新完成",
                    config.name
                );
            }
            // 依赖就绪后推动全部配置刷新, 迁移后的旧数据借此补齐刷新时间
            let count = ConfigBiz::new(context.clone()).sync(None).await?;
            debug!("[启动] 推送配置刷新数量: {count}");
        } else {
            if stage != Some(Stage::Dependency) {
                stage = Some(Stage::Dependency);
                info!(
                    "[启动] 配置「{}」的依赖尚未刷新完成, 等待依赖同步",
                    config.name
                );
            }
            dependencies.push_pending();
        }

        tokio::time::sleep(STEP).await;
    }
}

/// 等待阶段, 仅用于避免同一阶段反复写日志。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    /// 依赖尚未刷新完成
    Dependency,
    /// 依赖已就绪, 等待配置自身刷新完成
    Config,
}

/// 读取选中配置, 已被删除时返回错误。
async fn find(context: &Arc<DbContext>, id: i64) -> Result<Config> {
    ConfigService::new(context.clone())
        .find(id)
        .await?
        .ok_or_else(|| anyhow!("配置不存在: {id}"))
}

/// 读取配置依赖的订阅与规则。
///
/// 配置未选择订阅或引用的订阅已不存在时返回错误; 已被删除的规则不再需要同步,
/// 因此不参与依赖判断。
async fn dependencies(context: &Arc<DbContext>, config: &Config) -> Result<Dependencies> {
    let Some(subscribe_id) = config.subscribe_id else {
        bail!("配置「{}」未选择订阅", config.name);
    };
    let Some(subscribe) = SubscribeService::new(context.clone())
        .find(subscribe_id)
        .await?
    else {
        bail!("配置「{}」引用的订阅不存在: {subscribe_id}", config.name);
    };

    let ids: Vec<i64> = config
        .rule_direct_ids
        .iter()
        .chain(&config.rule_proxy_ids)
        .chain(&config.rule_reject_ids)
        .copied()
        .collect();
    let rules = RuleService::new(context.clone())
        .list()
        .await?
        .into_iter()
        .filter(|rule| ids.contains(&rule.id))
        .map(|rule| (rule.id, rule.refresh_time))
        .collect();

    Ok(Dependencies {
        subscribe: subscribe.id,
        subscribe_refresh: subscribe.refresh_time,
        rules,
    })
}

/// 配置依赖的订阅与规则。
struct Dependencies {
    /// 订阅 id
    subscribe: i64,
    /// 订阅的刷新时间
    subscribe_refresh: i64,
    /// 引用且仍然存在的规则 id 与刷新时间
    rules: Vec<(i64, i64)>,
}

impl Dependencies {
    /// 依赖是否都已刷新完成。
    fn ready(&self) -> bool {
        self.subscribe_refresh != 0 && self.rules.iter().all(|(_, refresh)| *refresh != 0)
    }

    /// 推送尚未刷新完成的依赖, 已就绪的不重复同步。
    fn push_pending(&self) {
        if self.subscribe_refresh == 0 {
            SubscribeSync::instance().push(self.subscribe);
        }

        let sync = RuleSync::instance();
        for (id, refresh) in &self.rules {
            if *refresh == 0 {
                sync.push(*id);
            }
        }
    }
}
