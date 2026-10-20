mod directories;
mod kernel;

use gpui_kit::base::{StyledExt as _, v_flex};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::*;

use directories::Directories;
use kernel::Kernel;

/// 各卡片之间的间隔。
const GAP: Rems = rems(1.);

/// 设置页面：由若干独立的分区纵向排列组成。
///
/// 页面只负责分区的顺序与间距，每个分区自己负责读取数据与呈现；
/// 当前有内核（内核设置、内核配置）与数据路径。
pub struct SettingsPage {
    /// 数据路径分区，目录在页面构造时读取一次。
    directories: Directories,
    /// 内核分区，设置由分区自己读取与保存。
    kernel: Entity<Kernel>,
}

impl SettingsPage {
    /// 创建页面。
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            directories: Directories::new(),
            kernel: cx.new(|cx| Kernel::new(window, cx)),
        }
    }
}

impl Render for SettingsPage {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        // 分区数量随功能增长，页面自己滚动，避免内容超出后被内容区裁掉。
        v_flex()
            .id("settings-page")
            .size_full()
            .min_h_0()
            .gap(GAP)
            .overflow_y_scrollbar()
            .child(self.kernel.clone())
            .child(self.directories.clone())
    }
}
