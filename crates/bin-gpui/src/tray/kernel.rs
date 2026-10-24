//! 托盘菜单里的内核动作。
//!
//! 托盘菜单没有配置下拉框，因此「启动内核」只能用上次使用的配置；
//! 没有可用配置时菜单项保持置灰，由 [`refresh`] 判断。

use gpui_kit::*;
use lib_biz::kernel::Status;
use service_core::holder::settings::SettingsHolder;

use crate::services::{self, AppServices};

use super::refresh_kernel;

/// 按内核的当前状态刷新菜单项。
pub fn refresh(cx: &mut App) {
    let data = services::kernel_data();
    let running = data.status == Status::Running;
    // 启动中/停止中不允许再次操作；停止态还要求有可启动的配置
    let busy = !matches!(data.status, Status::Running | Status::Stopped);
    let has_config = SettingsHolder::run().selected.is_some();

    refresh_kernel(cx, running, !busy && (running || has_config));
}

/// 按当前状态启动或停止内核。
pub fn toggle(cx: &mut App) {
    let Some(AppServices::Ready(services)) = services::get(cx) else {
        return;
    };
    let services = services.clone();

    let running = services::kernel_data().status == Status::Running;
    let selected = SettingsHolder::run().selected;

    let runner = services.clone();
    cx.spawn(async move |cx| {
        let task = services.spawn(async move {
            match (running, selected) {
                (true, _) => runner.stop_kernel().await,
                (false, Some(id)) => runner.start_kernel(id).await,
                (false, None) => Ok(()),
            }
        });

        let result = match task.await {
            Ok(result) => result,
            Err(error) => Err(anyhow::anyhow!("内核任务异常: {error}")),
        };

        if let Err(error) = result {
            log::error!("[托盘] 切换内核状态失败: {error:#}");
        }
        cx.update(refresh);
    })
    .detach();
}
