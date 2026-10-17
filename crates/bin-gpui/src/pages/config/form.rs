use gpui_kit::base::{StyledExt as _, h_flex};
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::combobox::ComboboxState;
use gpui_kit::component::searchable_list::{SearchableListDelegate as _, SearchableListItem};
use gpui_kit::component::select::SearchableVec;
use gpui_kit::*;

use types_core::dto::config::ConfigUpsertDTO;
use types_core::entity::config::Config;
use types_core::entity::rule::Rule;
use types_core::entity::subscribe::Subscribe;

use crate::utils::format;

/// 订阅下拉的一行：名称 + 剩余流量，选择时能区分同名订阅。
#[derive(Clone)]
pub struct SubscribeItem {
    id: SharedString,
    name: SharedString,
    traffic: SharedString,
}

impl SearchableListItem for SubscribeItem {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.name.clone()
    }

    fn render(&self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        row(&self.name, &self.traffic, cx)
    }

    fn value(&self) -> &Self::Value {
        &self.id
    }
}

/// 订阅下拉的数据源。
pub type SubscribeDelegate = SearchableVec<SubscribeItem>;

/// 由订阅列表构建下拉数据源。
pub fn subscribe_delegate(items: &[Subscribe]) -> SubscribeDelegate {
    SearchableVec::new(
        items
            .iter()
            .map(|item| SubscribeItem {
                id: item.id.clone().into(),
                name: item.name.clone().into(),
                traffic: format::remaining(item.max, item.upload + item.download).into(),
            })
            .collect::<Vec<_>>(),
    )
}

/// 规则下拉的一行：名称 + 规则数量。
#[derive(Clone)]
pub struct RuleItem {
    id: SharedString,
    name: SharedString,
    count: SharedString,
}

impl SearchableListItem for RuleItem {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.name.clone()
    }

    fn render(&self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        row(&self.name, &self.count, cx)
    }

    fn value(&self) -> &Self::Value {
        &self.id
    }
}

/// 规则下拉的数据源。
pub type RuleDelegate = SearchableVec<RuleItem>;

/// 由规则列表构建下拉数据源。
pub fn rule_delegate(items: &[Rule]) -> RuleDelegate {
    SearchableVec::new(
        items
            .iter()
            .map(|item| RuleItem {
                id: item.id.clone().into(),
                name: item.name.clone().into(),
                count: format!("{} 条", item.count).into(),
            })
            .collect::<Vec<_>>(),
    )
}

/// 一行下拉内容：主要文字 + 次要信息。
fn row(name: &SharedString, hint: &SharedString, cx: &App) -> impl IntoElement {
    h_flex().gap_2().child(name.clone()).child(
        div()
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(hint.clone()),
    )
}

/// 建立多选规则下拉的状态，`selected` 为已选规则 id。
pub fn rule_state(
    delegate: RuleDelegate,
    selected: &[String],
    window: &mut Window,
    cx: &mut App,
) -> Entity<ComboboxState<RuleDelegate>> {
    let indices = selected
        .iter()
        .filter_map(|id| delegate.position(&SharedString::from(id.clone())))
        .collect::<Vec<_>>();

    cx.new(|cx| {
        ComboboxState::new(delegate, indices, window, cx)
            .multiple(true)
            .searchable(true)
    })
}

/// 表单里除输入框以外的字段。
///
/// 布尔开关与区域多选都在弹窗内就地修改，因此需要一份可变的实体承载，
/// 下拉框的选中值直接读它们自己的状态，不在这里重复保存。
pub struct ConfigForm {
    pub tun: bool,
    pub ipv6: bool,
    pub fake_ip: bool,
    pub geo_cn: bool,
    /// 包含: 是否包含无区域节点
    pub include_area_non: bool,
    /// 包含区域，ISO 代码
    pub include_area: Vec<String>,
    /// 排除区域，ISO 代码
    pub exclude_area: Vec<String>,
}

impl ConfigForm {
    /// 由实体裁剪出编辑所需的字段。
    pub fn of(config: &Config) -> Self {
        Self {
            tun: config.tun,
            ipv6: config.ipv6,
            fake_ip: config.fake_ip,
            geo_cn: config.geo_cn,
            include_area_non: config.include_area_non,
            include_area: config.include_area.clone(),
            exclude_area: config.exclude_area.clone(),
        }
    }

    /// 新增配置的默认值，与业务层的默认入参一致。
    pub fn new() -> Self {
        let dto = ConfigUpsertDTO::default();
        Self {
            tun: dto.tun,
            ipv6: dto.ipv6,
            fake_ip: dto.fake_ip,
            geo_cn: dto.geo_cn,
            include_area_non: dto.include_area_non,
            include_area: dto.include_area,
            exclude_area: dto.exclude_area,
        }
    }
}
