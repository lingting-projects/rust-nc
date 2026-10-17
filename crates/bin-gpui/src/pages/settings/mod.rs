mod directories;

use gpui_kit::base::{StyledExt as _, v_flex};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::*;

use directories::Directories;

/// 各分区之间的间隔。
const GAP: Rems = rems(1.);

/// 设置页面：由若干独立的分区纵向排列组成。
///
/// 页面只负责分区的顺序与间距，每个分区自己负责读取数据与呈现；
/// 当前只有数据路径一个分区。
pub struct SettingsPage {
    /// 数据路径分区，目录在页面构造时读取一次。
    directories: Directories,
}

impl SettingsPage {
    /// 创建页面。
    pub fn new(_: &mut Window, _: &mut Context<Self>) -> Self {
        Self {
            directories: Directories::new(),
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
            .child(self.directories.clone())
    }
}
