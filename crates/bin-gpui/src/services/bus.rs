//! 总线安装。
//!
//! 业务层写库后发布数据更新事件，页面订阅后自行重读数据。
//! 总线实例在 `lib-core` 里全局唯一，这里只把它交给界面：
//! 装上 GPUI 的分发者，再放进 `App` 的全局状态供页面订阅。

use std::sync::Arc;

use gpui_kit::*;
use lingting_gpui_kit::{BusAppExt as _, GpuiBackgroundDispatcher};

/// 把全局总线装到界面状态里。
///
/// 必须在创建页面之前调用：页面在构造时订阅事件，装得晚了会漏掉启动阶段的通知。
pub fn init(cx: &mut App) {
    let bus = lib_core::bus::bus().clone();
    bus.set_dispatcher(Arc::new(GpuiBackgroundDispatcher::new(cx)));
    cx.install_bus(bus);
}
